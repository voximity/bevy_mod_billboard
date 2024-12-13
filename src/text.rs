use crate::pipeline::{RenderBillboardImage, RenderBillboardMesh};
use crate::utils::calculate_billboard_uniform;
use crate::{BillboardDepth, BillboardLockAxis, BillboardText};
use bevy::color::palettes;
use bevy::prelude::*;
use bevy::render::mesh::{Indices, PrimitiveTopology};
use bevy::render::render_asset::RenderAssetUsages;
use bevy::render::sync_world::RenderEntity;
use bevy::render::Extract;
use bevy::sprite::Anchor;
use bevy::text::{
    ComputedTextBlock, CosmicFontSystem, FontAtlasSets, PositionedGlyph, SwashCache, TextBounds,
    TextLayoutInfo, TextPipeline, TextReader, YAxisOrientation,
};
use bevy::utils::{HashMap, HashSet};
use smallvec::SmallVec;

// Uses this as reference
// https://github.com/bevyengine/bevy/blob/v0.11.2/crates/bevy_text/src/text2d.rs

#[derive(Component, Copy, Clone, Debug, Reflect, Deref, Default)]
#[reflect(Component)]
pub struct BillboardTextBounds(pub TextBounds);

// TODO: Maybe use something like { Single(Group), Multi(SmallVec<[Group; 1]>) }, benchmark it
#[derive(Component, Clone, Debug, Deref, DerefMut, Default, Reflect)]
#[reflect(Component)]
pub struct BillboardTextHandles(pub SmallVec<[BillboardTextHandleGroup; 1]>);

#[derive(Clone, Debug, Default, Reflect)]
pub struct BillboardTextHandleGroup {
    mesh: Handle<Mesh>,
    image: Handle<Image>,
}

pub fn extract_billboard_text(
    mut commands: Commands,
    mut previous_len: Local<usize>,
    billboard_text_query: Extract<
        Query<(
            RenderEntity,
            &ViewVisibility,
            &GlobalTransform,
            &Transform,
            &BillboardTextHandles,
            &BillboardDepth,
            Option<&BillboardLockAxis>,
        )>,
    >,
) {
    let mut batch = Vec::with_capacity(*previous_len);

    for (render_entity, visibility, global_transform, transform, handles, &depth, lock_axis) in
        &billboard_text_query
    {
        if !visibility.get() {
            continue;
        }

        let uniform = calculate_billboard_uniform(global_transform, transform, lock_axis);

        for handle_group in handles.iter() {
            // TODO: this will overwrite the render entity if we try to
            // TODO: add multiple handles in the same extraction!
            batch.push((
                render_entity,
                (
                    uniform,
                    RenderBillboardMesh {
                        id: handle_group.mesh.id(),
                    },
                    RenderBillboardImage {
                        id: handle_group.image.id(),
                    },
                    RenderBillboard {
                        depth,
                        lock_axis: lock_axis.copied(),
                    },
                ),
            ));
        }
    }

    *previous_len = batch.len();
    commands.insert_batch(batch);
}

pub fn update_billboard_text_layout(
    mut queue: Local<HashSet<Entity>>,
    mut images: ResMut<Assets<Image>>,
    mut meshes: ResMut<Assets<Mesh>>,
    fonts: Res<Assets<Font>>,
    mut texture_atlases: ResMut<Assets<TextureAtlasLayout>>,
    mut font_atlas_set_storage: ResMut<FontAtlasSets>,
    mut text_pipeline: ResMut<TextPipeline>,
    mut font_system: ResMut<CosmicFontSystem>,
    mut swash_cache: ResMut<SwashCache>,
    mut text_query: Query<
        (
            Entity,
            &mut TextLayoutInfo,
            Ref<TextLayout>,
            Ref<BillboardTextBounds>,
            Ref<Anchor>,
            &mut BillboardTextHandles,
            &mut ComputedTextBlock,
        ),
        With<BillboardText>,
    >,
    mut text_reader: TextReader<BillboardText>,
) {
    const SCALE_FACTOR: f64 = 1.0;

    for (entity, mut info, layout, bounds, anchor, mut handles, mut computed) in &mut text_query {
        if layout.is_changed()
            || bounds.is_changed()
            || anchor.is_changed()
            || computed.needs_rerender()
            || queue.remove(&entity)
        {
            let text_bounds = if layout.linebreak == LineBreak::NoWrap {
                TextBounds::UNBOUNDED
            } else {
                bounds.0
            };

            match text_pipeline.queue_text(
                &mut info,
                &fonts,
                text_reader.iter(entity),
                SCALE_FACTOR,
                &layout,
                text_bounds,
                &mut font_atlas_set_storage,
                &mut texture_atlases,
                &mut images,
                YAxisOrientation::BottomToTop,
                computed.as_mut(),
                &mut font_system,
                &mut swash_cache,
            ) {
                Err(TextError::NoSuchFont) => {
                    error!("Missing font (could still be loading)");
                    queue.insert(entity);
                    continue;
                }
                Err(err @ TextError::FailedToAddGlyph(_)) => {
                    panic!("Fatal error when processing text: {err}.");
                }
                Err(err @ TextError::FailedToGetGlyphImage(_)) => {
                    panic!("Fatal error when processing text: {err}.");
                }
                Ok(_) => (),
            };

            let text_anchor = -(anchor.as_vec() + 0.5);
            let alignment_translation = info.size * text_anchor;

            let length = info.glyphs.len();
            let mut textures = HashMap::new();

            for glyph in &info.glyphs {
                // TODO: Maybe with clever caching, could be possible to get rid of or_insert_with,
                // TODO: though I don't know how much of a gain it would be. Just keeping this as a note.
                let entry = textures
                    .entry(glyph.atlas_info.texture.clone_weak())
                    .or_insert_with(|| {
                        (
                            Vec::with_capacity(length),
                            (
                                texture_atlases
                                    .get(&glyph.atlas_info.texture_atlas)
                                    .expect("Atlas should exist"),
                                glyph.atlas_info.texture.clone_weak(),
                            ),
                        )
                    });

                entry.0.push(glyph.clone());
            }

            handles.clear();

            for (glyphs, (atlas, texture)) in textures.into_values() {
                let mut positions = Vec::with_capacity(info.glyphs.len() * 4);
                let mut uvs = Vec::with_capacity(info.glyphs.len() * 4);
                let mut colors = Vec::with_capacity(info.glyphs.len() * 4);
                let mut indices = Vec::with_capacity(info.glyphs.len() * 6);

                let mut color = palettes::css::WHITE.to_f32_array();
                let mut current_span = usize::MAX;

                for PositionedGlyph {
                    position,
                    size,
                    atlas_info,
                    span_index,
                    ..
                } in glyphs
                {
                    let index = positions.len() as u32;
                    let position = position + alignment_translation;

                    let half_size = size / 2.0;
                    let top_left = position - half_size;
                    let bottom_right = position + half_size;

                    positions.extend([
                        [top_left.x, top_left.y, 0.0],
                        [top_left.x, bottom_right.y, 0.0],
                        [bottom_right.x, bottom_right.y, 0.0],
                        [bottom_right.x, top_left.y, 0.0],
                    ]);

                    let URect { min, max } = atlas.textures[atlas_info.location.glyph_index];
                    let atlas_size = atlas.size.as_vec2();
                    let min = min.as_vec2() / atlas_size;
                    let max = max.as_vec2() / atlas_size;

                    uvs.extend([
                        [min.x, max.y],
                        [min.x, min.y],
                        [max.x, min.y],
                        [max.x, max.y],
                    ]);

                    if span_index != current_span {
                        color = text_reader
                            .get_color(entity, span_index)
                            .unwrap()
                            .to_linear()
                            .to_f32_array();
                        current_span = span_index;
                    }

                    colors.extend([color, color, color, color]);

                    indices.extend([index, index + 2, index + 1, index, index + 3, index + 2]);
                }

                let mut mesh = Mesh::new(
                    PrimitiveTopology::TriangleList,
                    RenderAssetUsages::default(),
                );

                mesh.insert_attribute(Mesh::ATTRIBUTE_POSITION, positions);
                mesh.insert_attribute(Mesh::ATTRIBUTE_UV_0, uvs);
                mesh.insert_attribute(Mesh::ATTRIBUTE_COLOR, colors);

                mesh.insert_indices(Indices::U32(indices));

                handles.push(BillboardTextHandleGroup {
                    mesh: meshes.add(mesh),
                    image: texture,
                });
            }
        }
    }
}

// TODO: Use EntityHash with EntityHashMap in 0.12 for extracted.
// The related code is removed, but this todo is helpful for future.

#[derive(Component)]
pub struct RenderBillboard {
    pub depth: BillboardDepth,
    pub lock_axis: Option<BillboardLockAxis>,
}
