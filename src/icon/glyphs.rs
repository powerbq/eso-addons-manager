use lucide_icons::Icon;

pub const NAME: Icon = Icon::CaseLower;
pub const AUTHOR: Icon = Icon::User;
pub const UPDATED: Icon = Icon::Clock;
pub const DOWNLOADS: Icon = Icon::Download;
pub const MONTHLY_DOWNLOADS: Icon = Icon::Calendar;
pub const FAVOURITES: Icon = Icon::Star;
pub const STATUS: Icon = Icon::Layers3;
pub const ASCENDING: Icon = Icon::MoveUp;
pub const DESCENDING: Icon = Icon::MoveDown;
pub const INSTALL: Icon = Icon::CirclePlus;
pub const INSTALLED_AUTO: Icon = Icon::CircleCheck;
pub const REMOVE: Icon = Icon::Trash2;
pub const COLLAPSED: Icon = Icon::ChevronRight;
pub const EXPANDED: Icon = Icon::ChevronDown;
pub const OPEN_PAGE: Icon = Icon::ExternalLink;
pub const CHECKED: Icon = Icon::Check;
pub const MENU: Icon = Icon::Menu;

#[allow(dead_code)]
pub const ALL: [Icon; 17] = [
    NAME,
    AUTHOR,
    UPDATED,
    DOWNLOADS,
    MONTHLY_DOWNLOADS,
    FAVOURITES,
    STATUS,
    ASCENDING,
    DESCENDING,
    INSTALL,
    INSTALLED_AUTO,
    REMOVE,
    COLLAPSED,
    EXPANDED,
    OPEN_PAGE,
    CHECKED,
    MENU,
];
