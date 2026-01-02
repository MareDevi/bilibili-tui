use std::io;
use bilibili_tui::ui::App;


fn main() -> io::Result<()> {
    let mut terminal = ratatui::init(); 
    
    let app_result = App::new().run(&mut terminal);

    ratatui::restore();

    app_result
}