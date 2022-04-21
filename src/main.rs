use bevy::prelude::*;
use bevy_asset_loader::{AssetCollection, AssetCollectionApp};

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .init_collection::<ModelAssets>()
        .insert_resource(AmbientLight {
            color: Color::WHITE,
            brightness: 1.0 / 5.0f32,
        })
        .add_startup_system(setup)
        .add_system(animate_light_direction)
        .run();
}

#[derive(AssetCollection)]
struct ModelAssets {
    #[asset(path = "models/ground_pathSplit.glb#Scene0")]
    ground_path_split: Handle<Scene>,
}

fn setup(mut commands: Commands, models: Res<ModelAssets>) {
    commands.spawn_scene(models.ground_path_split.clone());
    commands.spawn_bundle(PerspectiveCameraBundle {
        transform: Transform::from_xyz(0.7, 0.7, 1.0).looking_at(Vec3::new(0.0, 0.3, 0.0), Vec3::Y),
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
