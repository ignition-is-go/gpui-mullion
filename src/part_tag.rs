//! Names for Mullion's chrome, for a harness that measures where each part
//! lands on screen.
//!
//! Mullion does not record anything itself. An application that wants the
//! parts named installs a [`PartTagger`] with [`set_part_tagger`]; Mullion then
//! passes each part through it with a stable id. With no tagger installed the
//! parts render untouched and no id string is built.
//!
//! Each id is `mullion.` followed by the part's debug selector:
//!
//! | id | part |
//! | --- | --- |
//! | `mullion.pane:<pane>` | a pane, including its activity bar |
//! | `mullion.pane-header:<pane>` | a pane's header row |
//! | `mullion.activity-bar:<pane>` | a pane's activity rail |
//! | `mullion.activity:<pane>:<activity>` | an activity rail item |
//! | `mullion.activity-category:<pane>:<category>` | an activity category row |
//! | `mullion.split-handle:<split>` | the visible bar between two panes |
//! | `mullion.workspace:<workspace>` | a workspace tab |

use gpui::{AnyElement, App, Global, IntoElement, SharedString};

/// Wraps a part's element with its id. It must not change the part's layout.
pub type PartTagger = fn(AnyElement, SharedString) -> AnyElement;

struct GlobalPartTagger(PartTagger);
impl Global for GlobalPartTagger {}

/// Installs the tagger every Mullion view passes its parts through.
pub fn set_part_tagger(cx: &mut App, tagger: PartTagger) {
    cx.set_global(GlobalPartTagger(tagger));
}

/// Passes `element` through the installed tagger as `mullion.<selector>`.
pub(crate) fn tag_part(
    cx: &App,
    element: impl IntoElement,
    selector: impl FnOnce() -> String,
) -> AnyElement {
    let element = element.into_any_element();
    match cx.try_global::<GlobalPartTagger>() {
        Some(tagger) => (tagger.0)(element, format!("mullion.{}", selector()).into()),
        None => element,
    }
}
