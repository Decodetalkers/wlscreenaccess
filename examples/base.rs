use std::error::Error;
use wlscreenaccess::{color_pick,screenshot};
// Although we use `async-std` here, you can use any async runtime of choice.
#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    let a = screenshot().await?;
    dbg!(a);
    let b = color_pick().await?;
    let b = b.to_rgb();
    dbg!(b);
    Ok(())
}
