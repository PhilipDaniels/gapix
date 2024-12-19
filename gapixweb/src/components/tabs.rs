use std::fmt::Display;

use maud::{html, Markup};

/// The list of tabs at the top of the page, used to specify which tab is
/// selected.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Tabs {
    Rides,
    Segments,
    Controls,
    Jobs,
    Settings,
}

/// Returns the markup for the list of tabs at the top of the page.
/// `selected_tab` specifies which tab is currently selected.
pub fn tabs(selected_tab: Tabs) -> Markup {
    let html = html! {
        div role="tablist" class="tabs tabs-lifted" {
            (tab(Tabs::Rides, selected_tab))
            (tab(Tabs::Segments, selected_tab))
            (tab(Tabs::Controls, selected_tab))
            (tab(Tabs::Jobs, selected_tab))
            (tab(Tabs::Settings, selected_tab))
        }
    };

    html
}

/// Creates a single tab.
fn tab(tab: Tabs, selected_tab: Tabs) -> Markup {
    let classes = if tab == selected_tab {
        "tab tab-active"
    } else {
        "tab"
    };

    html! {
        a role="tab" class=(classes) href=(tab.href()) { (tab.to_string()) }
    }
}

impl Display for Tabs {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match *self {
            Tabs::Rides => write!(f, "Rides"),
            Tabs::Segments => write!(f, "Segments"),
            Tabs::Controls => write!(f, "Controls"),
            Tabs::Jobs => write!(f, "Jobs"),
            Tabs::Settings => write!(f, "Settings"),
        }
    }
}

impl Tabs {
    /// Returns the href that a particular tab will route to.
    pub fn href(self) -> &'static str {
        match self {
            Tabs::Rides => "/rides",
            Tabs::Segments => "/segments",
            Tabs::Controls => "/controls",
            Tabs::Jobs => "/jobs",
            Tabs::Settings => "/settings",
        }
    }
}