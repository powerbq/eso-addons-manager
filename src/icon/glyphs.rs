use lucide_icons::Icon;

pub const NAME: Icon = Icon::CaseLower;
pub const AUTHOR: Icon = Icon::User;
pub const UPDATED: Icon = Icon::Clock;
pub const DOWNLOADS: Icon = Icon::Download;
pub const MONTHLY_DOWNLOADS: Icon = Icon::Calendar;
pub const FAVOURITES: Icon = Icon::Star;
pub const ASCENDING: Icon = Icon::MoveUp;
pub const DESCENDING: Icon = Icon::MoveDown;
pub const INSTALL: Icon = Icon::CirclePlus;
pub const REMOVE: Icon = Icon::Trash2;
pub const COLLAPSED: Icon = Icon::ChevronRight;
pub const EXPANDED: Icon = Icon::ChevronDown;
pub const OPEN_PAGE: Icon = Icon::ExternalLink;
pub const CHECKED: Icon = Icon::Check;
pub const MENU: Icon = Icon::Menu;

#[allow(dead_code)]
pub const ALL: [Icon; 15] = [
    NAME,
    AUTHOR,
    UPDATED,
    DOWNLOADS,
    MONTHLY_DOWNLOADS,
    FAVOURITES,
    ASCENDING,
    DESCENDING,
    INSTALL,
    REMOVE,
    COLLAPSED,
    EXPANDED,
    OPEN_PAGE,
    CHECKED,
    MENU,
];
