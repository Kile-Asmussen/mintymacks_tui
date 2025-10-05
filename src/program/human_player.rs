

pub struct HumanPlayer {
    pub name: String,
    pub elo: Option<u16>,
    pub title: Option<Title>,
}

pub enum Title {
    CM, FM, IM, GM
}