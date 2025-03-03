# YAGE
Yet Another Game Engine

This game engine is slightly based off of `javax.swing` API's, in the sense that there are event/listener models

for example, 

one could write:
```rust
use yage_core::{Engine, EngineBuilder, component::{sync::{AsyncComponent, AsyncDynamicComponent}, RenderContext}};
use yage_components::mesh::Mesh2D;
use yage_net::TcpStream;

use core::pin::Pin;
use core::task::{Context, Poll};

#[derive(Status)]
enum PlayerStatus {
  Healing,
  Killed,
  Fighting,
  Chilling,
}

struct GameState {
  player_health: usize,
  player_status: PlayerStatus,
  game_stream: TcpStream,
}

// the `yage::component` tag provides a set of listeners, a builder, and other utilities
// TODO: other utilities?
#[yage::component]
struct PlayerComponent {
  weapon: Weapon,
  dimensions: Dimensions,
  player_mesh: Mesh2D,
}

impl AsyncComponent<GameState> for PlayerComponent {
  type Error = ();
  

  fn poll_draw(self: Pin<&mut Self>, cx: &mut Context<'_>, render_context: &mut RenderContext<GameState>) -> Poll<Result<(), ()>> {
      self.weapon.poll_draw(cx, render_context)?;
      self.player_mesh.poll_draw(cx, render_context)?;
      Ok(())
  }
}

impl AsyncDynamicComponent<GameState> for PlayerComponent {
  fn poll_update(self: Pin<&mut Self, cx: &mut Context<'_>, render_context: &mut RenderContext<GameState>) -> Poll<Result<(), ()>> {
    // update logic here, read stream for networking updates, etc
    let mut stream = render_context.state().game_stream;
    let mut buf = SmallVec::with_capacity(core::mem::size_of<{some serializeable struct}>());
    let mut buf = ReadBuf::new(&mut *buf);
    match core::task::ready!(Pin::new(&mut *stream).poll_read(cx, buf) {
      Ok(()) => {},
      Err(e) => return Err(())
    }
    // other logic, etc...
  }   
}

#[yage::main]
async fn main() {
  let mut engine = EngineBuilder::new()
  .with_state(GameState {
    player_health: 100,
    player_status: PlayerStatus::Chilling,
    game_stream: TcpStream::connect().await.unwrap()
  })
  .build()
  .unwrap();

  engine.spawn(|assets| {
    PlayerComponent {
      weapon: Weapon::Sword,
      dimensions: Dimensions {
        len: 200,
        width: 200
      },
      player_mesh: Mesh2D::load(assets.file("player.png"))
    }
  }).unwrap();

  engine.run().await.unwrap()
}
 
```

