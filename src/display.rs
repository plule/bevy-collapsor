use crate::components::*;
use bevy::prelude::*;
use bevy_mod_picking::Hover;

pub struct DisplayPlugin;

impl Plugin for DisplayPlugin {
    fn build(&self, app: &mut App) {
        app.add_system(pick_draw_tile)
            .add_system(draw_rules)
            .add_system(draw_map)
            .add_system(apply_coordinate)
            .add_system(animate_light_direction);
    }
}

fn pick_draw_tile(
    mut query: Query<(&mut DrawTile, &OptionalTile, &Hover)>,
    selection: Res<TileSelection>,
) {
    for (mut draw_tile, map_tile, hover) in query.iter_mut() {
        match hover.hovered() {
            // When hovered, display the selection tile
            true => {
                let tile = OptionalTile::new(selection.make_tile());
                if draw_tile.tile != tile {
                    draw_tile.tile = tile;
                }
            }
            // When not hovered, display the tile from the map
            false => {
                if draw_tile.tile != *map_tile {
                    draw_tile.tile = map_tile.clone();
                }
            }
        }
    }
}

fn draw_rules(
    query: Query<(Entity, &DrawTile), Changed<DrawTile>>,
    mut commands: Commands,
    models: Res<ModelAssets>,
    rules: Res<Rules>,
) {
    for (entity, draw_tile) in query.iter() {
        let mut entity = commands.entity(entity);
        entity.despawn_descendants();

        if let Some(tile) = &draw_tile.tile.tile {
            entity.with_children(|parent| {
                let prototype = &rules.prototypes[tile.prototype_index];
                let model = prototype.model.clone();
                let transform = Transform::from_rotation(tile.orientation.clone().into())
                    .with_translation(Vec3::new(0.0, 0.2, 0.0));

                parent
                    .spawn_bundle((transform, GlobalTransform::default()))
                    .with_children(|tile| {
                        tile.spawn_scene(model);
                        tile.spawn_bundle(PbrBundle {
                            material: models.up_cube_mat.clone(),
                            mesh: models.up_cube_mesh.clone(),
                            transform: Transform::from_translation(-Vec3::Z / 2.5),
                            ..Default::default()
                        });
                    });
            });
        };
    }
}

fn draw_map(
    query: Query<(Entity, &TileSuperposition), Changed<TileSuperposition>>,
    mut commands: Commands,
    models: Res<ModelAssets>,
    rules: Res<Rules>,
) {
    for (entity, multi_tile) in query.iter() {
        let mut entity = commands.entity(entity);
        entity.despawn_descendants();

        match multi_tile.tiles.len() {
            0 => {
                entity.with_children(|tile| {
                    tile.spawn_bundle(PbrBundle {
                        mesh: models.impossible_mesh.clone(),
                        material: models.impossible_mat.clone(),
                        ..Default::default()
                    });
                });
            }
            1 => {
                let tile = multi_tile.tiles.iter().next().unwrap();
                let prototype = &rules.prototypes[tile.prototype_index];
                let model = prototype.model.clone();
                let transform = Transform::from_rotation(tile.orientation.clone().into());
                entity.with_children(|tile| {
                    tile.spawn_bundle(TransformBundle::from_transform(transform))
                        .with_children(|tile| {
                            tile.spawn_scene(model);
                        });
                });
            }
            _ => {
                entity.with_children(|tile| {
                    tile.spawn_bundle(PbrBundle {
                        mesh: models.undecided_mesh.clone(),
                        material: models.undecided_mat.clone(),
                        ..Default::default()
                    });
                });
            }
        }
    }
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
            time.seconds_since_startup() as f32 * std::f32::consts::TAU / 20.0,
            -std::f32::consts::FRAC_PI_4,
        );
    }
}