use bevy::prelude::*;
use bevy_inspector_egui::{Inspectable, RegisterInspectable, WorldInspectorPlugin};
use bevy_mod_picking::*;
use rand::Rng;

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_plugin(WorldInspectorPlugin::new())
        .add_plugins(DefaultPickingPlugins)
        .insert_resource(AmbientLight {
            color: Color::WHITE,
            brightness: 1.0 / 5.0f32,
        })
        .init_resource::<ModelAssets>()
        .add_startup_system(setup)
        .register_inspectable::<Coordinates>()
        .add_system(apply_coordinate)
        .add_system(animate_light_direction)
        .run();
}

#[derive(Default)]
struct ModelAssets {
    models: Vec<Handle<Scene>>,
}

fn setup(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    mut models: ResMut<ModelAssets>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    commands
        .spawn_bundle(PerspectiveCameraBundle {
            transform: Transform::from_xyz(-10.0, 10.0, 0.0)
                .looking_at(Vec3::new(0.0, 0.0, 0.0), Vec3::Y),
            ..default()
        })
        .insert_bundle(PickingCameraBundle::default());
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

    let mut rng = rand::thread_rng();
    let mut map = Map::new(64, 64);

    let tile_type_nb = map.tile_models.len();
    for tile in map.tiles.iter_mut() {
        *tile = rng.gen_range(0..tile_type_nb);
    }

    models.models = map
        .tile_models
        .iter()
        .map(|path| asset_server.load(path))
        .collect();

    let pick_mesh = meshes.add(Mesh::from(shape::Cube { size: 1.0 }));
    let pick_mat = materials.add(StandardMaterial {
        base_color: Color::rgba(1.0, 1.0, 1.0, 0.1),
        alpha_mode: AlphaMode::Blend,
        ..Default::default()
    });

    for i in 0..map.tile_models.len() {
        let model = models.models[i].clone();
        commands
            .spawn_bundle(PbrBundle {
                material: pick_mat.clone(),
                mesh: pick_mesh.clone(),
                transform: Transform::default(),
                global_transform: GlobalTransform::default(),
                ..Default::default()
            })
            .insert_bundle(PickableBundle::default())
            .insert_bundle((
                Name::from(format!("tile proto {i}")),
                Coordinates::new(-5, 2 * (i as i32) - 4),
            ))
            .with_children(|tile| {
                tile.spawn_scene(model);
            });
    }

    /*for x in 0..map.width {
        for y in 0..map.height {
            let idx = map.tile_at(x, y);
            let model = models.models[idx].clone();
            commands
                .spawn_bundle((
                    Name::from(format!("{x}:{y}")),
                    Transform::default(),
                    GlobalTransform::default(),
                    Coordinates::new(x, y),
                ))
                .with_children(|tile| {
                    tile.spawn_scene(model);
                });
        }
    }*/
}

fn apply_coordinate(mut query: Query<(&mut Transform, &Coordinates), Changed<Coordinates>>) {
    for (mut transform, coordinates) in query.iter_mut() {
        transform.translation.x = coordinates.x as f32;
        transform.translation.y = 0.;
        transform.translation.z = coordinates.y as f32;
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

struct Map {
    pub tile_models: Vec<String>,
    pub tiles: Vec<usize>,
    pub width: usize,
    pub height: usize,
}

impl Map {
    fn new(width: usize, height: usize) -> Self {
        Self {
            tile_models: vec![
                "models/ground_grass.glb#Scene0".to_string(),
                "models/ground_pathBend.glb#Scene0".to_string(),
                "models/ground_pathCross.glb#Scene0".to_string(),
                "models/ground_pathEndClosed.glb#Scene0".to_string(),
                "models/ground_pathSplit.glb#Scene0".to_string(),
                "models/ground_pathStraight.glb#Scene0".to_string(),
            ],
            tiles: vec![0; width * height],
            width,
            height,
        }
    }

    fn tile_at(&self, x: usize, y: usize) -> usize {
        self.tiles[x + y * self.width]
    }
}

#[derive(Component, Inspectable, Default)]
struct Coordinates {
    pub x: i32,
    pub y: i32,
}

impl Coordinates {
    fn new(x: i32, y: i32) -> Self {
        Self { x, y }
    }
}

#[derive(Component, Inspectable)]
struct TileType {
    file: String,
}
