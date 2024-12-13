pub mod pipeline;
pub mod plugin;
pub mod text;
pub mod texture;
mod utils;

use crate::text::{BillboardTextBounds, BillboardTextHandles};
use bevy::prelude::*;
use bevy::render::extract_component::ExtractComponent;
use bevy::sprite::Anchor;
use bevy::text::{TextRoot, TextSpanAccess};

pub(self) const BILLBOARD_SHADER_HANDLE: Handle<Shader> =
    Handle::weak_from_u128(12823766040132746076);

/// Marker component for a billboarded texture.
///
/// Additionally insert a [`BillboardMesh`] to function.
#[derive(Clone, Component, Default, Reflect)]
#[reflect(Component)]
#[require(Billboard, BillboardMesh, Transform, Visibility)]
pub struct BillboardTexture(pub Handle<Image>);

/// Marker component for billboarded text.
///
/// Optionally insert [`TextSpan`] children to render separate sections.
///
/// # Warning
///
/// This component is incompatible with [`Text`] and [`Text2d`]!
/// Bevy will attempt to render the `Text` in the UI and `Text2D` as 2D
/// text, corrupting the internal `TextLayoutInfo` used by billboarding.
///
/// If you are not using `TextSpan` children, set the `String` field of
/// this struct.
#[derive(Clone, Component, Default)]
#[require(
    Billboard,
    BillboardTextBounds,
    BillboardTextHandles,
    TextLayout,
    TextFont,
    TextColor,
    Anchor,
    Transform,
    Visibility
)]
pub struct BillboardText(pub String);

impl BillboardText {
    pub fn new(value: impl Into<String>) -> Self {
        Self(value.into())
    }
}

impl TextRoot for BillboardText {}

impl TextSpanAccess for BillboardText {
    fn read_span(&self) -> &str {
        self.0.as_str()
    }
    fn write_span(&mut self) -> &mut String {
        &mut self.0
    }
}

impl From<&str> for BillboardText {
    fn from(value: &str) -> Self {
        Self(String::from(value))
    }
}

impl From<String> for BillboardText {
    fn from(value: String) -> Self {
        Self(value)
    }
}

#[derive(Clone, Component, Reflect, Default)]
#[reflect(Component)]
pub struct BillboardMesh(pub Handle<Mesh>);

#[derive(Clone, Copy, Component, Debug, Reflect)]
pub struct BillboardDepth(pub bool);

impl Default for BillboardDepth {
    fn default() -> Self {
        Self(true)
    }
}

#[derive(Default, Clone, Copy, Component, ExtractComponent, Debug, Reflect)]
#[require(BillboardDepth)]
pub struct Billboard;

#[derive(Default, Clone, Copy, Component, Debug, Reflect)]
pub struct BillboardLockAxis {
    pub y_axis: bool,
    pub rotation: bool,
}

impl BillboardLockAxis {
    pub fn new() -> Self {
        Default::default()
    }

    pub fn from_lock_y(y_axis: bool) -> Self {
        Self {
            y_axis,
            ..default()
        }
    }

    pub fn from_lock_rotation(rotation: bool) -> Self {
        Self {
            rotation,
            ..default()
        }
    }

    pub fn with_lock_y(mut self, y_axis: bool) -> Self {
        self.y_axis = y_axis;
        self
    }

    pub fn with_lock_rotation(mut self, rotation: bool) -> Self {
        self.rotation = rotation;
        self
    }
}

pub mod prelude {
    pub use crate::{
        plugin::BillboardPlugin, text::BillboardTextBounds, BillboardDepth, BillboardLockAxis,
        BillboardMesh, BillboardText, BillboardTexture,
    };
}
