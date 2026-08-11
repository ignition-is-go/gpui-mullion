use std::fmt;

use crate::{PaneCommand, PaneDirection, PaneLayout, PaneRotation, SplitDirection};
use serde::{Deserialize, Serialize};

/// A platform-neutral keyboard event snapshot.
///
/// Serde represents this as an object with `key`, `control`, `alt`, `shift`,
/// and `meta` fields. Deserialization does not fill omitted modifiers; callers
/// producing configuration must serialize all five fields.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct KeyStroke {
    /// Key name reported by the frontend, compared case-insensitively.
    pub key: String,
    /// Whether the Control modifier was held.
    pub control: bool,
    /// Whether the Alt/Option modifier was held.
    pub alt: bool,
    /// Whether the Shift modifier was held.
    pub shift: bool,
    /// Whether the platform modifier (Command on macOS) was held.
    pub meta: bool,
}

impl KeyStroke {
    /// Creates a stroke with `key` and no modifiers.
    pub fn new(key: impl Into<String>) -> Self {
        Self {
            key: key.into(),
            control: false,
            alt: false,
            shift: false,
            meta: false,
        }
    }
}

/// A key plus its exact modifier set.
///
/// The serialized object uses the same five fields as [`KeyStroke`]. Modifier
/// booleans are requirements, not a bit mask, and omitted fields are errors.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct KeyChord {
    /// Key name to match, compared case-insensitively after normalization.
    pub key: String,
    /// Whether an exact match requires Control.
    pub control: bool,
    /// Whether an exact match requires Alt/Option.
    pub alt: bool,
    /// Whether an exact match requires Shift.
    pub shift: bool,
    /// Whether an exact match requires the platform modifier.
    pub meta: bool,
}

impl KeyChord {
    /// Creates a chord for `key` with no required modifiers.
    pub fn new(key: impl Into<String>) -> Self {
        Self {
            key: key.into(),
            control: false,
            alt: false,
            shift: false,
            meta: false,
        }
    }

    /// Adds Control to the required modifier set.
    pub fn control(mut self) -> Self {
        self.control = true;
        self
    }

    /// Adds Alt/Option to the required modifier set.
    pub fn alt(mut self) -> Self {
        self.alt = true;
        self
    }

    /// Adds Shift to the required modifier set.
    pub fn shift(mut self) -> Self {
        self.shift = true;
        self
    }

    /// Adds the platform modifier to the required modifier set.
    pub fn meta(mut self) -> Self {
        self.meta = true;
        self
    }

    /// Returns whether `stroke` has the normalized key and exact modifier set.
    pub fn matches(&self, stroke: &KeyStroke) -> bool {
        normalize_key(&self.key) == normalize_key(&stroke.key)
            && self.control == stroke.control
            && self.alt == stroke.alt
            && self.shift == stroke.shift
            && self.meta == stroke.meta
    }
    /// GPUI-compatible normalized chord spelling (for example `ctrl-alt-left`).
    pub fn normalized(&self) -> String {
        let mut parts = Vec::new();
        if self.control {
            parts.push("ctrl".to_owned());
        }
        if self.alt {
            parts.push("alt".to_owned());
        }
        if self.shift {
            parts.push("shift".to_owned());
        }
        if self.meta {
            parts.push("platform".to_owned());
        }
        let key = match normalize_key(&self.key).as_str() {
            "arrowleft" => "left".to_owned(),
            "arrowright" => "right".to_owned(),
            "arrowup" => "up".to_owned(),
            "arrowdown" => "down".to_owned(),
            other => other.to_owned(),
        };
        parts.push(key);
        parts.join("-")
    }
}

impl fmt::Display for KeyChord {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.control {
            f.write_str("Ctrl+")?;
        }
        if self.alt {
            f.write_str("Alt+")?;
        }
        if self.shift {
            f.write_str("Shift+")?;
        }
        if self.meta {
            f.write_str("Meta+")?;
        }
        let key = match normalize_key(&self.key).as_str() {
            "space" => "Space".into(),
            "arrowleft" => "←".into(),
            "arrowright" => "→".into(),
            "arrowup" => "↑".into(),
            "arrowdown" => "↓".into(),
            "escape" => "Esc".into(),
            other => other.to_uppercase(),
        };
        f.write_str(&key)
    }
}

/// Returns the canonical comparison spelling for a frontend key name.
///
/// Case is folded through Unicode lowercase conversion, and the legacy
/// `" "` and `"Spacebar"` spellings become `"space"`.
pub fn normalize_key(key: &str) -> String {
    match key {
        " " | "Spacebar" => "space".into(),
        other => other.to_lowercase(),
    }
}

/// One command sequence after a keymap's optional prefix.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct MullionKeyBinding {
    /// Ordered chords entered after the map's optional prefix.
    ///
    /// The vector is serialized as the `sequence` array. Public constructors
    /// and [`MullionKeymap::bind_sequence`] are the supported way to avoid an
    /// empty sequence, although deserialization itself does not validate it.
    pub sequence: Vec<KeyChord>,
    /// Command dispatched after every chord in [`Self::sequence`] matches.
    pub command: PaneCommand,
}

impl MullionKeyBinding {
    /// Creates a one-chord binding.
    pub fn new(chord: KeyChord, command: PaneCommand) -> Self {
        Self {
            sequence: vec![chord],
            command,
        }
    }

    /// Creates a binding from an already ordered post-prefix sequence.
    ///
    /// This constructor preserves an empty vector; adding bindings through
    /// [`MullionKeymap::bind_sequence`] instead ignores empty sequences.
    pub fn from_sequence(sequence: Vec<KeyChord>, command: PaneCommand) -> Self {
        Self { sequence, command }
    }
}

/// Pane keymap with an optional prefix.
///
/// Direct maps resolve bindings immediately. Prefixed maps may contain more
/// than one chord after their prefix, which supports terminal-muxer-style and
/// application-specific command modes without constraining Mullion's default.
///
/// Serde stores the private state as an object with `prefix`, `bindings`, and
/// `ignore_editable_targets`. This stable configuration contract includes the
/// editable-target policy; deserialization performs no binding validation.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct MullionKeymap {
    prefix: Option<KeyChord>,
    bindings: Vec<MullionKeyBinding>,
    ignore_editable_targets: bool,
}

impl MullionKeymap {
    /// Creates an empty prefixed map that ignores editable targets.
    pub fn new(prefix: KeyChord) -> Self {
        Self {
            prefix: Some(prefix),
            bindings: Vec::new(),
            ignore_editable_targets: true,
        }
    }

    /// Create a map whose bindings do not require a prefix.
    pub fn unprefixed() -> Self {
        Self {
            prefix: None,
            bindings: Vec::new(),
            ignore_editable_targets: true,
        }
    }

    /// Mullion's direct, modifier-based default command map.
    pub fn mullion() -> Self {
        use PaneCommand::*;
        use PaneDirection::*;

        let mut map = Self::unprefixed();
        for (key, direction) in [
            ("ArrowLeft", Left),
            ("ArrowDown", Down),
            ("ArrowUp", Up),
            ("ArrowRight", Right),
        ] {
            map.bind(KeyChord::new(key).alt(), Focus(direction));
            map.bind(KeyChord::new(key).alt().shift(), Move(direction));
            map.bind(KeyChord::new(key).control().shift(), Swap(direction));
            map.bind(KeyChord::new(key).control().alt(), Resize(direction));
        }

        map.bind(KeyChord::new("PageDown").alt(), FocusNext);
        map.bind(KeyChord::new("PageUp").alt(), FocusPrevious);
        map.bind(KeyChord::new("Home").alt(), FocusFirst);
        map.bind(KeyChord::new("End").alt(), FocusLast);
        for index in 0..9 {
            map.bind(
                KeyChord::new((index + 1).to_string()).alt(),
                FocusIndex(index),
            );
        }

        map.bind(
            KeyChord::new("ArrowRight").control().alt().shift(),
            Split(SplitDirection::Horizontal),
        );
        map.bind(
            KeyChord::new("ArrowDown").control().alt().shift(),
            Split(SplitDirection::Vertical),
        );
        map.bind(KeyChord::new("Backspace").control().shift(), Close);
        map.bind(KeyChord::new("Enter").control().shift(), ToggleZoom);

        map.bind(KeyChord::new("PageUp").control().shift(), SwapPrevious);
        map.bind(KeyChord::new("PageDown").control().shift(), SwapNext);

        map.bind(
            KeyChord::new("h").control().alt(),
            SetParentSplitDirection(SplitDirection::Horizontal),
        );
        map.bind(
            KeyChord::new("v").control().alt(),
            SetParentSplitDirection(SplitDirection::Vertical),
        );
        map.bind(
            KeyChord::new("o").control().alt(),
            ToggleParentSplitDirection,
        );
        map.bind(KeyChord::new("=").control().alt(), Balance);

        map.bind(
            KeyChord::new("[").control().alt(),
            Rotate(PaneRotation::Backward),
        );
        map.bind(
            KeyChord::new("]").control().alt(),
            Rotate(PaneRotation::Forward),
        );

        for (key, layout) in [
            ("1", PaneLayout::EvenHorizontal),
            ("2", PaneLayout::EvenVertical),
            ("3", PaneLayout::MainHorizontal),
            ("4", PaneLayout::MainVertical),
            ("5", PaneLayout::Tiled),
        ] {
            map.bind(KeyChord::new(key).control().alt(), ApplyLayout(layout));
        }
        map
    }

    /// An opt-in `Ctrl+B` map for applications whose users expect tmux.
    pub fn tmux() -> Self {
        use PaneCommand::*;
        use PaneDirection::*;

        let mut map = Self::new(KeyChord::new("b").control());
        for (key, direction) in [
            ("h", Left),
            ("j", Down),
            ("k", Up),
            ("l", Right),
            ("ArrowLeft", Left),
            ("ArrowDown", Down),
            ("ArrowUp", Up),
            ("ArrowRight", Right),
        ] {
            map.bind(KeyChord::new(key), Focus(direction));
        }

        map.bind(KeyChord::new("o"), FocusNext);
        map.bind(KeyChord::new(";"), FocusPrevious);
        for index in 0..9 {
            map.bind(KeyChord::new((index + 1).to_string()), FocusIndex(index));
        }

        // tmux's `%` creates a left/right split and `"` a top/bottom split.
        map.bind(
            KeyChord::new("%").shift(),
            Split(SplitDirection::Horizontal),
        );
        map.bind(KeyChord::new("\"").shift(), Split(SplitDirection::Vertical));
        map.bind(KeyChord::new("x"), Close);
        map.bind(KeyChord::new("z"), ToggleZoom);
        map.bind(KeyChord::new("Space"), ToggleParentSplitDirection);
        map.bind(KeyChord::new("e"), Balance);

        map.bind(KeyChord::new("{").shift(), SwapPrevious);
        map.bind(KeyChord::new("}").shift(), SwapNext);
        map.bind(KeyChord::new("o").control(), Rotate(PaneRotation::Forward));
        map.bind(KeyChord::new("o").alt(), Rotate(PaneRotation::Backward));

        for (key, direction) in [
            ("ArrowLeft", Left),
            ("ArrowDown", Down),
            ("ArrowUp", Up),
            ("ArrowRight", Right),
        ] {
            map.bind(KeyChord::new(key).shift(), Move(direction));
            map.bind(KeyChord::new(key).alt(), Swap(direction));
            map.bind(KeyChord::new(key).control(), Resize(direction));
        }

        for (key, direction) in [("h", Left), ("j", Down), ("k", Up), ("l", Right)] {
            map.bind(KeyChord::new(key).shift(), Move(direction));
            map.bind(KeyChord::new(key).alt(), Swap(direction));
            map.bind(KeyChord::new(key).control(), Resize(direction));
        }

        for (key, layout) in [
            ("1", PaneLayout::EvenHorizontal),
            ("2", PaneLayout::EvenVertical),
            ("3", PaneLayout::MainHorizontal),
            ("4", PaneLayout::MainVertical),
            ("5", PaneLayout::Tiled),
        ] {
            map.bind(KeyChord::new(key).alt(), ApplyLayout(layout));
        }
        map
    }

    /// Returns the chord required before every binding, or `None` for a direct map.
    pub fn prefix(&self) -> Option<&KeyChord> {
        self.prefix.as_ref()
    }

    /// Returns bindings in insertion order.
    pub fn bindings(&self) -> &[MullionKeyBinding] {
        &self.bindings
    }

    /// Replace any existing binding for `chord`.
    pub fn bind(&mut self, chord: KeyChord, command: PaneCommand) {
        self.bind_sequence([chord], command);
    }

    /// Replace any existing binding for the exact post-prefix sequence.
    pub fn bind_sequence(
        &mut self,
        sequence: impl IntoIterator<Item = KeyChord>,
        command: PaneCommand,
    ) {
        let sequence: Vec<_> = sequence.into_iter().collect();
        if sequence.is_empty() {
            return;
        }
        self.bindings.retain(|binding| binding.sequence != sequence);
        self.bindings
            .push(MullionKeyBinding::from_sequence(sequence, command));
    }

    /// Adds or replaces a one-chord binding and returns the map.
    pub fn with_binding(mut self, chord: KeyChord, command: PaneCommand) -> Self {
        self.bind(chord, command);
        self
    }

    /// Adds or replaces a post-prefix chord sequence and returns the map.
    ///
    /// An empty iterator leaves the map unchanged.
    pub fn with_sequence(
        mut self,
        sequence: impl IntoIterator<Item = KeyChord>,
        command: PaneCommand,
    ) -> Self {
        self.bind_sequence(sequence, command);
        self
    }

    /// Configure whether sequences originating in inputs, textareas, selects,
    /// or content-editable elements may be captured. Defaults to `false`.
    pub fn capture_editable_targets(mut self, capture: bool) -> Self {
        self.ignore_editable_targets = !capture;
        self
    }

    /// Display the full key sequence for a command, including its prefix when
    /// the map has one.
    pub fn sequence_for(&self, command: PaneCommand) -> Option<String> {
        self.bindings
            .iter()
            .find(|binding| binding.command == command)
            .map(|binding| {
                self.prefix
                    .iter()
                    .chain(binding.sequence.iter())
                    .map(ToString::to_string)
                    .collect::<Vec<_>>()
                    .join(", ")
            })
    }

    /// GPUI-compatible full sequences, including the optional prefix.
    pub fn normalized_sequences(&self) -> Vec<(String, PaneCommand)> {
        self.bindings
            .iter()
            .map(|binding| {
                let sequence = self
                    .prefix
                    .iter()
                    .chain(binding.sequence.iter())
                    .map(KeyChord::normalized)
                    .collect::<Vec<_>>()
                    .join(" ");
                (sequence, binding.command)
            })
            .collect()
    }

    /// Whether keyboard adapters should capture shortcuts from editable controls.
    pub fn captures_editable_targets(&self) -> bool {
        !self.ignore_editable_targets
    }

    /// Classifies a post-prefix stroke sequence against the configured bindings.
    ///
    /// Exact matches take precedence over a longer binding with the same prefix.
    pub fn match_sequence(&self, strokes: &[KeyStroke]) -> KeySequenceMatch {
        if let Some(binding) = self.bindings.iter().find(|binding| {
            binding.sequence.len() == strokes.len()
                && binding
                    .sequence
                    .iter()
                    .zip(strokes)
                    .all(|(chord, stroke)| chord.matches(stroke))
        }) {
            return KeySequenceMatch::Command(binding.command);
        }

        if self.bindings.iter().any(|binding| {
            binding.sequence.len() > strokes.len()
                && binding
                    .sequence
                    .iter()
                    .zip(strokes)
                    .all(|(chord, stroke)| chord.matches(stroke))
        }) {
            KeySequenceMatch::Pending
        } else {
            KeySequenceMatch::NoMatch
        }
    }
}

/// Result of matching a post-prefix stroke sequence against a keymap.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum KeySequenceMatch {
    /// The complete sequence resolves to this command.
    Command(PaneCommand),
    /// The strokes are a proper prefix of at least one binding.
    Pending,
    /// No binding starts with the strokes.
    NoMatch,
}

impl Default for MullionKeymap {
    fn default() -> Self {
        Self::mullion()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sequence(keys: &[&str]) -> Vec<KeyStroke> {
        keys.iter().map(|key| KeyStroke::new(*key)).collect()
    }

    fn stroke(chord: KeyChord) -> KeyStroke {
        KeyStroke {
            key: chord.key,
            control: chord.control,
            alt: chord.alt,
            shift: chord.shift,
            meta: chord.meta,
        }
    }

    #[test]
    fn mullion_default_uses_direct_modifier_combinations() {
        let map = MullionKeymap::default();
        assert_eq!(map.prefix(), None);
        assert_eq!(
            map.match_sequence(&[stroke(KeyChord::new("ArrowLeft").alt())]),
            KeySequenceMatch::Command(PaneCommand::Focus(PaneDirection::Left))
        );
        assert_eq!(
            map.match_sequence(&[stroke(KeyChord::new("ArrowLeft").alt().shift())]),
            KeySequenceMatch::Command(PaneCommand::Move(PaneDirection::Left))
        );
        assert_eq!(
            map.match_sequence(&[stroke(KeyChord::new("ArrowRight").control().alt().shift())]),
            KeySequenceMatch::Command(PaneCommand::Split(SplitDirection::Horizontal))
        );
    }

    #[test]
    fn mullion_default_binds_the_entire_static_command_catalog() {
        let map = MullionKeymap::default();
        for command in PaneCommand::catalog() {
            assert!(
                map.sequence_for(command).is_some(),
                "missing default key sequence for {command:?}"
            );
        }
        for index in 0..9 {
            assert!(map.sequence_for(PaneCommand::FocusIndex(index)).is_some());
        }
    }

    #[test]
    fn tmux_prefix_and_navigation_are_bound() {
        let map = MullionKeymap::tmux();
        assert!(map.prefix().unwrap().matches(&KeyStroke {
            key: "b".into(),
            control: true,
            alt: false,
            shift: false,
            meta: false,
        }));
        assert_eq!(
            map.match_sequence(&sequence(&["h"])),
            KeySequenceMatch::Command(PaneCommand::Focus(PaneDirection::Left))
        );
    }

    #[test]
    fn chords_require_an_exact_modifier_set() {
        let chord = KeyChord::new("ArrowLeft").control();
        let mut stroke = KeyStroke::new("ArrowLeft");
        assert!(!chord.matches(&stroke));
        stroke.control = true;
        assert!(chord.matches(&stroke));
        stroke.shift = true;
        assert!(!chord.matches(&stroke));
    }

    #[test]
    fn space_spellings_match() {
        assert!(KeyChord::new("Space").matches(&KeyStroke::new(" ")));
    }

    #[test]
    fn rebinding_replaces_the_existing_chord() {
        let mut map = MullionKeymap::new(KeyChord::new("b").control());
        map.bind(KeyChord::new("x"), PaneCommand::Close);
        map.bind(KeyChord::new("x"), PaneCommand::ToggleZoom);
        assert_eq!(map.bindings().len(), 1);
        assert_eq!(
            map.match_sequence(&sequence(&["x"])),
            KeySequenceMatch::Command(PaneCommand::ToggleZoom)
        );
    }

    #[test]
    fn rebinding_replaces_only_the_exact_sequence() {
        let mut map = MullionKeymap::new(KeyChord::new("m").control());
        map.bind_sequence(
            [KeyChord::new("m"), KeyChord::new("ArrowLeft")],
            PaneCommand::Move(PaneDirection::Left),
        );
        map.bind_sequence(
            [KeyChord::new("m"), KeyChord::new("ArrowLeft")],
            PaneCommand::Swap(PaneDirection::Left),
        );
        map.bind(KeyChord::new("m"), PaneCommand::FocusNext);

        assert_eq!(map.bindings().len(), 2);
        assert_eq!(
            map.match_sequence(&sequence(&["m", "ArrowLeft"])),
            KeySequenceMatch::Command(PaneCommand::Swap(PaneDirection::Left))
        );
    }

    #[test]
    fn keymaps_round_trip_as_configuration() {
        let map = MullionKeymap::default();
        let json = serde_json::to_string(&map).unwrap();
        let restored: MullionKeymap = serde_json::from_str(&json).unwrap();
        assert_eq!(restored.prefix(), map.prefix());
        assert_eq!(restored.bindings(), map.bindings());
        assert_eq!(
            restored.ignore_editable_targets,
            map.ignore_editable_targets
        );
    }
    #[test]
    fn normalized_sequences_include_custom_prefix_and_multichords() {
        let map = MullionKeymap::new(KeyChord::new("m").control()).with_sequence(
            [KeyChord::new("g"), KeyChord::new("ArrowLeft").alt()],
            PaneCommand::FocusFirst,
        );
        assert_eq!(
            map.normalized_sequences(),
            vec![("ctrl-m g alt-left".into(), PaneCommand::FocusFirst)]
        );
    }

    #[test]
    fn editable_capture_is_explicit_and_serialized() {
        let map = MullionKeymap::default().capture_editable_targets(true);
        assert!(map.captures_editable_targets());
        let restored: MullionKeymap =
            serde_json::from_str(&serde_json::to_string(&map).unwrap()).unwrap();
        assert!(restored.captures_editable_targets());
    }
}
