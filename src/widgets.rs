use crossterm::event::Event;


trait Widget {
    fn terminal_event(ev: Event);
    fn render();
}