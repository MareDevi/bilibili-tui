use bilibili_tui::ui::App;
use std::io;

#[tokio::main]
async fn main() -> io::Result<()> {
    let mut terminal = ratatui::init();

    let app_result = App::new().run(&mut terminal).await;

    ratatui::restore();

    app_result
}
