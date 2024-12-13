use bevy::color::palettes;
use bevy::prelude::*;
use bevy_mod_billboard::prelude::*;

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_plugins(BillboardPlugin)
        .add_systems(Startup, (setup_billboard, setup_scene))
        .add_systems(Update, rotate_camera)
        .run();
}

fn setup_billboard(mut commands: Commands, asset_server: Res<AssetServer>) {
    let text_font =
        TextFont::from_font(asset_server.load("FiraSans-Regular.ttf")).with_font_size(60.0);

    commands
        .spawn((
            BillboardText::default(),
            TextLayout::new_with_justify(JustifyText::Center),
            Transform::from_scale(Vec3::splat(0.0085)).looking_at(Vec3::splat(5.0), Vec3::Y),
            BillboardLockAxis::from_lock_rotation(true),
        ))
        .with_child((
            TextSpan::new("LOCKED"),
            text_font.clone(),
            TextColor(Color::Srgba(palettes::css::ORANGE)),
        ))
        .with_child((TextSpan::new(" text"), text_font, TextColor(Color::WHITE)));
}

// Important bits are above, the code below is for camera, reference cube and rotation

#[derive(Component)]
#[require(Transform)]
pub struct CameraHolder;

fn setup_scene(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    commands.spawn(CameraHolder).with_children(|parent| {
        parent.spawn((
            Camera3d::default(),
            Transform::from_xyz(5., 0., 0.).looking_at(Vec3::ZERO, Vec3::Y),
        ));
    });

    commands.spawn((
        Mesh3d(meshes.add(Cuboid::default())),
        MeshMaterial3d(materials.add(Color::Srgba(palettes::css::GRAY))),
        Transform::from_translation(Vec3::NEG_Y),
    ));
}

fn rotate_camera(mut camera: Query<&mut Transform, With<CameraHolder>>, time: Res<Time>) {
    let mut camera = camera.single_mut();

    camera.rotate_y(time.delta_secs());
}
