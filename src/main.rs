use anyhow::Result;
use trowser::App;

#[tokio::main]
async fn main() -> Result<()> {
    App::new()?.run().await?;
    // let response = reqwest::get("http://localhost:8000").await?;
    // let text = response.text().await?;
    // let node = trowser::Parser::parse(text);
    // println!("{:?}", node);
    Ok(())
}
