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
pub fn tabs(selected_tab: Tabs, selected_tab_content: &Markup) -> Markup {
    let html = html! {
        div role="tablist" class="tabs tabs-lifted" {
            (tab(Tabs::Rides, selected_tab, selected_tab_content))
            (tab(Tabs::Segments, selected_tab, selected_tab_content))
            (tab(Tabs::Controls, selected_tab, selected_tab_content))
            (tab(Tabs::Jobs, selected_tab, selected_tab_content))
            (tab(Tabs::Settings, selected_tab, selected_tab_content))
        }
    };

    html
}

/// Creates a single tab.
fn tab(tab: Tabs, selected_tab: Tabs, selected_tab_content: &Markup) -> Markup {
    if tab == selected_tab {
        html! {
            a role="tab" class="tab tab-active" href=(tab.href()) { (tab.to_string()) }
            div role="tabpanel" class="tab-content bg-base-100 border-base-300 rounded-box p-6" {
                (selected_tab_content)
            }
        }
    } else {
        html! {
            a role="tab" class="tab" href=(tab.href()) { (tab.to_string()) }
        }
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