use bevy::prelude::*;
use bevy_asset_loader::{AssetCollection, AssetCollectionApp};
use bevy_inspector_egui::{Inspectable, RegisterInspectable, WorldInspectorPlugin};

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .init_collection::<ModelAssets>()
        .insert_resource(AmbientLight {
            color: Color::WHITE,
            brightness: 1.0 / 5.0f32,
        })
        .add_plugin(WorldInspectorPlugin::new())
        .add_startup_system(setup)
        .register_inspectable::<Coordinates>()
        .add_system(apply_coordinate)
        .add_system(animate_light_direction)
        .run();
}

#[derive(AssetCollection)]
struct ModelAssets {
    #[asset(path = "models/ground_pathSplit.glb#Scene0")]
    ground_path_split: Handle<Scene>,
}

#[derive(Component, Inspectable, Default)]
struct Coordinates {
    pub x: i32,
    pub z: i32,
}

impl Coordinates {
    fn new(x: i32, z: i32) -> Self {
        Self { x, z }
    }
}

fn setup(mut commands: Commands, models: Res<ModelAssets>) {
    commands.spawn_bundle(PerspectiveCameraBundle {
        transform: Transform::from_xyz(0.7, 5.0, 1.0).looking_at(Vec3::new(0.0, 0.0, 0.0), Vec3::Y),
        ..default()
    });
    const HALF_SIZE: f32 = 1.0;
    commands.spawn_bundle(DirectionalLightBundle {
        directional_light: DirectionalLight {
            shadow_projection: OrthographicProjection {
                left: -HALF_SIZE,
                right: HALF_SIZE,
                bottom: -HALF_SIZE,
                top: HALF_SIZE,
                near: -10.0 * HALF_SIZE,
                far: 10.0 * HALF_SIZE,
                ..default()
            },
            shadows_enabled: true,
            ..default()
        },
        ..default()
    });

    for x in -10..10 {
        for z in -10..10 {
            commands
                .spawn_bundle((
                    Name::from(format!("{x}:{z}")),
                    Transform::default(),
                    GlobalTransform::default(),
                    Coordinates::new(x, z),
                ))
                .with_children(|tile| {
                    tile.spawn_scene(models.ground_path_split.clone());
                });
        }
    }
}

fn apply_coordinate(mut query: Query<(&mut Transform, &Coordinates), Changed<Coordinates>>) {
    for (mut transform, coordinates) in query.iter_mut() {
        transform.translation.x = coordinates.x as f32;
        transform.translation.y = 0.;
        transform.translation.z = coordinates.z as f32;
    }
}

fn animate_light_direction(
    time: Res<Time>,
    mut query: Query<&mut Transform, With<DirectionalLight>>,
) {
    for mut transform in query.iter_mut() {
        transform.rotation = Quat::from_euler(
            EulerRot::ZYX,
            0.0,
            time.seconds_since_startup() as f32 * std::f32::consts::TAU / 10.0,
            -std::f32::consts::FRAC_PI_4,
        );
    }
}
