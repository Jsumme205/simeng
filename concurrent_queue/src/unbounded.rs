use core::cell::UnsafeCell;
use core::hint::spin_loop;
use core::mem::MaybeUninit;
use core::sync::atomic::{AtomicPtr, AtomicUsize, Ordering};

use crate::{AtomicExt, CachePadded, PopError, PushError, UnsafeCellExt};

const WRITE: usize = 1;
const READ: usize = 2;
const DESTROY: usize = 4;

const LAP: usize = 32;
const BLOCK_CAP: usize = LAP - 1;
const SHIFT: usize = 1;
const MARK_BIT: usize = 1;

struct Slot<T> {
    value: UnsafeCell<MaybeUninit<T>>,
    state: AtomicUsize,
}

impl<T> Slot<T> {
    const UNINIT: Self = Self {
        value: UnsafeCell::new(MaybeUninit::uninit()),
        state: AtomicUsize::new(0),
    };

    const fn uninit_block() -> [Self; BLOCK_CAP] {
        [const { Self::UNINIT }; BLOCK_CAP]
    }

    fn wait_write(&self) {
        while self.state.load(Ordering::Acquire) & WRITE == 0 {
            spin_loop();
        }
    }
}

struct Block<T> {
    next: AtomicPtr<Block<T>>,
    slots: [Slot<T>; BLOCK_CAP],
}

impl<T> Block<T> {
    const fn new() -> Self {
        Self {
            next: AtomicPtr::new(core::ptr::null_mut()),
            slots: Slot::uninit_block(),
        }
    }

    fn wait_next(&self) -> *mut Block<T> {
        loop {
            let next = self.next.load(Ordering::Acquire);
            if !next.is_null() {
                return next;
            }
            spin_loop();
        }
    }

    unsafe fn destroy(this: *mut Block<T>, start: usize) {
        for i in start..BLOCK_CAP {
            let slot = unsafe { (*this).slots.get_unchecked(i) };

            if slot.state.load(Ordering::Acquire) & READ == 0
                && slot.state.fetch_or(DESTROY, Ordering::AcqRel) & READ == 0
            {
                return;
            }
        }
        unsafe {
            __block_dealloc(this);
        }
    }
}

unsafe fn __block_dealloc<T>(this: *mut Block<T>) {
    unsafe { alloc::alloc::dealloc(this as *mut u8, core::alloc::Layout::new::<Block<T>>()) };
}

unsafe fn __block_alloc<T>() -> *mut Block<T> {
    let layout = core::alloc::Layout::new::<Block<T>>();
    let block = unsafe { alloc::alloc::alloc(layout) as *mut Block<T> };
    if block.is_null() {
        alloc::alloc::handle_alloc_error(layout)
    }
    block
}

struct Position<T> {
    index: AtomicUsize,
    block: AtomicPtr<Block<T>>,
}

pub(super) struct Unbounded<T> {
    head: CachePadded<Position<T>>,
    tail: CachePadded<Position<T>>,
}

impl<T> Unbounded<T> {
    pub const fn new() -> Self {
        Self {
            head: CachePadded(Position {
                index: AtomicUsize::new(0),
                block: AtomicPtr::new(core::ptr::null_mut()),
            }),
            tail: CachePadded(Position {
                index: AtomicUsize::new(0),
                block: AtomicPtr::new(core::ptr::null_mut()),
            }),
        }
    }

    pub(super) fn push(&self, value: T) -> Result<(), PushError<T>> {
        let mut tail = self.tail.index.load(Ordering::Acquire);
        let mut block = self.tail.block.load(Ordering::Acquire);
        let mut next_block = None;

        loop {
            // Check if the queue is closed.
            if tail & MARK_BIT != 0 {
                return Err(PushError::Closed(value));
            }

            // Calculate the offset of the index into the block.
            let offset = (tail >> SHIFT) % LAP;

            // If we reached the end of the block, wait until the next one is installed.
            if offset == BLOCK_CAP {
                spin_loop();
                tail = self.tail.index.load(Ordering::Acquire);
                block = self.tail.block.load(Ordering::Acquire);
                continue;
            }

            // If we're going to have to install the next block, allocate it in advance in order to
            // make the wait for other threads as short as possible.
            if offset + 1 == BLOCK_CAP && next_block.is_none() {
                next_block = Some(unsafe {
                    let ptr = __block_alloc::<T>();
                    ptr.write(Block::new());
                    ptr
                });
            }

            // If this is the first value to be pushed into the queue, we need to allocate the
            // first block and install it.
            if block.is_null() {
                let new = unsafe {
                    let ptr = __block_alloc::<T>();
                    ptr.write(Block::new());
                    ptr
                };

                if self
                    .tail
                    .block
                    .compare_exchange(block, new, Ordering::Release, Ordering::Relaxed)
                    .is_ok()
                {
                    self.head.block.store(new, Ordering::Release);
                    block = new;
                } else {
                    next_block = Some(new);
                    tail = self.tail.index.load(Ordering::Acquire);
                    block = self.tail.block.load(Ordering::Acquire);
                    continue;
                }
            }

            let new_tail = tail + (1 << SHIFT);

            // Try advancing the tail forward.
            match self.tail.index.compare_exchange_weak(
                tail,
                new_tail,
                Ordering::SeqCst,
                Ordering::Acquire,
            ) {
                Ok(_) => unsafe {
                    // If we've reached the end of the block, install the next one.
                    if offset + 1 == BLOCK_CAP {
                        let next_block = next_block.unwrap();
                        self.tail.block.store(next_block, Ordering::Release);
                        self.tail.index.fetch_add(1 << SHIFT, Ordering::Release);
                        (*block).next.store(next_block, Ordering::Release);
                    }

                    // Write the value into the slot.
                    let slot = (*block).slots.get_unchecked(offset);
                    slot.value.with_mut(|slot| {
                        slot.write(MaybeUninit::new(value));
                    });
                    slot.state.fetch_or(WRITE, Ordering::Release);
                    return Ok(());
                },
                Err(t) => {
                    tail = t;
                    block = self.tail.block.load(Ordering::Acquire);
                }
            }
        }
    }

    pub(super) fn pop(&self) -> Result<T, PopError> {
        let mut head = self.head.index.load(Ordering::Acquire);
        let mut block = self.head.block.load(Ordering::Acquire);

        loop {
            // Calculate the offset of the index into the block.
            let offset = (head >> SHIFT) % LAP;

            // If we reached the end of the block, wait until the next one is installed.
            if offset == BLOCK_CAP {
                spin_loop();
                head = self.head.index.load(Ordering::Acquire);
                block = self.head.block.load(Ordering::Acquire);
                continue;
            }

            let mut new_head = head + (1 << SHIFT);

            if new_head & MARK_BIT == 0 {
                crate::full_fence();
                let tail = self.tail.index.load(Ordering::Relaxed);

                // If the tail equals the head, that means the queue is empty.
                if head >> SHIFT == tail >> SHIFT {
                    // Check if the queue is closed.
                    if tail & MARK_BIT != 0 {
                        return Err(PopError::Closed);
                    } else {
                        return Err(PopError::Empty);
                    }
                }

                // If head and tail are not in the same block, set `MARK_BIT` in head.
                if (head >> SHIFT) / LAP != (tail >> SHIFT) / LAP {
                    new_head |= MARK_BIT;
                }
            }

            // The block can be null here only if the first push operation is in progress.
            if block.is_null() {
                spin_loop();
                head = self.head.index.load(Ordering::Acquire);
                block = self.head.block.load(Ordering::Acquire);
                continue;
            }

            // Try moving the head index forward.
            match self.head.index.compare_exchange_weak(
                head,
                new_head,
                Ordering::SeqCst,
                Ordering::Acquire,
            ) {
                Ok(_) => unsafe {
                    // If we've reached the end of the block, move to the next one.
                    if offset + 1 == BLOCK_CAP {
                        let next = (*block).wait_next();
                        let mut next_index = (new_head & !MARK_BIT).wrapping_add(1 << SHIFT);
                        if !(*next).next.load(Ordering::Relaxed).is_null() {
                            next_index |= MARK_BIT;
                        }

                        self.head.block.store(next, Ordering::Release);
                        self.head.index.store(next_index, Ordering::Release);
                    }

                    // Read the value.
                    let slot = (*block).slots.get_unchecked(offset);
                    slot.wait_write();
                    let value = slot.value.with_mut(|slot| slot.read().assume_init());

                    // Destroy the block if we've reached the end, or if another thread wanted to
                    // destroy but couldn't because we were busy reading from the slot.
                    if offset + 1 == BLOCK_CAP {
                        Block::destroy(block, 0);
                    } else if slot.state.fetch_or(READ, Ordering::AcqRel) & DESTROY != 0 {
                        Block::destroy(block, offset + 1);
                    }

                    return Ok(value);
                },
                Err(h) => {
                    head = h;
                    block = self.head.block.load(Ordering::Acquire);
                }
            }
        }
    }

    pub(super) fn len(&self) -> usize {
        loop {
            // Load the tail index, then load the head index.
            let mut tail = self.tail.index.load(Ordering::SeqCst);
            let mut head = self.head.index.load(Ordering::SeqCst);

            // If the tail index didn't change, we've got consistent indices to work with.
            if self.tail.index.load(Ordering::SeqCst) == tail {
                // Erase the lower bits.
                tail &= !((1 << SHIFT) - 1);
                head &= !((1 << SHIFT) - 1);

                // Fix up indices if they fall onto block ends.
                if (tail >> SHIFT) & (LAP - 1) == LAP - 1 {
                    tail = tail.wrapping_add(1 << SHIFT);
                }
                if (head >> SHIFT) & (LAP - 1) == LAP - 1 {
                    head = head.wrapping_add(1 << SHIFT);
                }

                // Rotate indices so that head falls into the first block.
                let lap = (head >> SHIFT) / LAP;
                tail = tail.wrapping_sub((lap * LAP) << SHIFT);
                head = head.wrapping_sub((lap * LAP) << SHIFT);

                // Remove the lower bits.
                tail >>= SHIFT;
                head >>= SHIFT;

                // Return the difference minus the number of blocks between tail and head.
                return tail - head - tail / LAP;
            }
        }
    }

    pub fn is_empty(&self) -> bool {
        let head = self.head.index.load(Ordering::SeqCst);
        let tail = self.tail.index.load(Ordering::SeqCst);
        head >> SHIFT == tail >> SHIFT
    }

    pub fn close(&self) -> bool {
        let tail = self.tail.index.fetch_or(MARK_BIT, Ordering::SeqCst);
        tail & MARK_BIT == 0
    }

    pub fn is_closed(&self) -> bool {
        self.tail.index.load(Ordering::SeqCst) & MARK_BIT != 0
    }
}

impl<T> Drop for Unbounded<T> {
    fn drop(&mut self) {
        let Self { head, tail } = self;
        let Position { index: head, block } = &mut **head;

        head.with_mut(|&mut mut head| {
            tail.index.with_mut(|&mut mut tail| {
                // Erase the lower bits.
                head &= !((1 << SHIFT) - 1);
                tail &= !((1 << SHIFT) - 1);

                unsafe {
                    // Drop all values between `head` and `tail` and deallocate the heap-allocated blocks.
                    while head != tail {
                        let offset = (head >> SHIFT) % LAP;

                        if offset < BLOCK_CAP {
                            // Drop the value in the slot.
                            block.with_mut(|block| {
                                let slot = (**block).slots.get_unchecked(offset);
                                slot.value.with_mut(|slot| {
                                    let value = &mut *slot;
                                    value.as_mut_ptr().drop_in_place();
                                });
                            });
                        } else {
                            // Deallocate the block and move to the next one.
                            block.with_mut(|block| {
                                let next_block = (**block).next.with_mut(|next| *next);
                                __block_dealloc(*block);
                                *block = next_block;
                            });
                        }

                        head = head.wrapping_add(1 << SHIFT);
                    }

                    // Deallocate the last remaining block.
                    block.with_mut(|block| {
                        if !block.is_null() {
                            __block_dealloc(*block);
                        }
                    });
                }
            });
        });
    }
}
