use bevy::prelude::*;
use bevy_inspector_egui::{Inspectable, RegisterInspectable, WorldInspectorPlugin};
use bevy_mod_picking::*;

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
        .init_resource::<SelectedTileProto>()
        .add_startup_system(setup)
        .register_inspectable::<Coordinates>()
        .register_inspectable::<MapTile>()
        .register_inspectable::<Palette>()
        .add_system(apply_coordinate)
        .add_system(animate_light_direction)
        .add_system(draw_map)
        .add_system_to_stage(CoreStage::PostUpdate, on_pick_event)
        .run();
}

#[derive(Default)]
struct ModelAssets {
    models: Vec<Handle<Scene>>,
}

#[derive(Default)]
struct SelectedTileProto {
    index: Option<usize>,
}

#[derive(Component, Inspectable)]
struct Palette {
    index: usize,
}

impl Palette {
    fn new(index: usize) -> Self {
        Self { index }
    }
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
            transform: Transform::from_xyz(-10.0, 10.0, 8.0)
                .looking_at(Vec3::new(4.0, 0.0, 8.0), Vec3::Y),
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

    let map = Map::new(16, 16);

    models.models = map
        .tile_models
        .iter()
        .map(|path| asset_server.load(path))
        .collect();

    let pick_mesh = meshes.add(Mesh::from(shape::Plane { size: 1.0 }));
    let pick_mat = materials.add(StandardMaterial {
        base_color: Color::rgba(1.0, 1.0, 1.0, 0.1),
        alpha_mode: AlphaMode::Blend,
        ..Default::default()
    });

    // Palette
    for i in 0..map.tile_models.len() {
        let model = models.models[i].clone();
        commands
            .spawn_bundle(PbrBundle {
                material: pick_mat.clone(),
                mesh: pick_mesh.clone(),
                ..Default::default()
            })
            .insert_bundle(PickableBundle::default())
            .insert_bundle((
                Name::from(format!("tile proto {i}")),
                Coordinates::new(-2, 2 * (i as i32)),
                Palette::new(i),
            ))
            .with_children(|tile| {
                tile.spawn_bundle((
                    Transform::from_xyz(0.0, 0.1, 0.0),
                    GlobalTransform::default(),
                ))
                .with_children(|tile| {
                    tile.spawn_scene(model);
                });
            });
    }

    // Rule map
    for x in 0..map.width {
        for y in 0..map.height {
            commands
                .spawn_bundle(PbrBundle {
                    material: pick_mat.clone(),
                    mesh: pick_mesh.clone(),
                    ..Default::default()
                })
                .insert_bundle((
                    Name::from(format!("{x}:{y}")),
                    Coordinates::new(x as i32, y as i32),
                    MapTile::default(),
                ))
                .insert_bundle(PickableBundle::default());
        }
    }
}

fn draw_map(
    query: Query<(Entity, &MapTile), Changed<MapTile>>,
    mut commands: Commands,
    models: Res<ModelAssets>,
) {
    for (entity, map_tile) in query.iter() {
        let mut entity = commands.entity(entity);
        entity.despawn_descendants();
        if let Some(index) = map_tile.tile_prototype {
            entity.with_children(|tile| {
                let model = models.models[index].clone();
                tile.spawn_scene(model);
            });
        };
    }
}

fn apply_coordinate(mut query: Query<(&mut Transform, &Coordinates), Changed<Coordinates>>) {
    for (mut transform, coordinates) in query.iter_mut() {
        transform.translation.x = coordinates.x as f32;
        transform.translation.y = 0.;
        transform.translation.z = coordinates.y as f32;
    }
}

fn on_pick_event(
    mut events: EventReader<PickingEvent>,
    mut selected: ResMut<SelectedTileProto>,
    palette_query: Query<&Palette>,
    mut rule_map_query: Query<&mut MapTile>,
) {
    for event in events.iter() {
        match event {
            PickingEvent::Selection(_) => (),
            PickingEvent::Hover(_) => (),
            PickingEvent::Clicked(e) => {
                match palette_query.get(*e) {
                    Ok(e) => selected.index = Some(e.index),
                    Err(_) => (),
                };
                match rule_map_query.get_mut(*e) {
                    Ok(mut e) => e.tile_prototype = selected.index,
                    Err(_) => (),
                };
            }
        }
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
    pub width: usize,
    pub height: usize,
}

#[derive(Component, Inspectable, Default)]
struct MapTile {
    pub tile_prototype: Option<usize>,
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
            width,
            height,
        }
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
