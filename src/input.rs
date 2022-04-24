use crate::components::*;
use bevy::{input::mouse::MouseWheel, prelude::*};
use bevy_mod_picking::{Hover, PickingEvent};

pub struct InputPlugin;

impl Plugin for InputPlugin {
    fn build(&self, app: &mut App) {
        let system_set = SystemSet::new()
            .with_system(pick_tile)
            .with_system(on_mouse_wheel)
            .with_system(palette_select);
        app.add_system_set_to_stage(CoreStage::PostUpdate, system_set);
    }
}

fn pick_tile(
    mut query: Query<(&mut OptionalTile, &Hover)>,
    selection: Res<TileSelection>,
    mouse_button_input: Res<Input<MouseButton>>,
    mut event_writer: EventWriter<RulesNeedUpdateEvent>,
) {
    let new_tile;
    if mouse_button_input.pressed(MouseButton::Left) {
        new_tile = selection.make_tile();
    } else if mouse_button_input.pressed(MouseButton::Right) {
        new_tile = None;
    } else {
        return;
    }
    let new_tile = OptionalTile::new(new_tile);

    let mut changed = false;
    for (mut map_tile, hover) in query.iter_mut() {
        if hover.hovered() && *map_tile != new_tile {
            *map_tile = new_tile.clone();
            changed = true;
        }
    }

    if changed {
        event_writer.send(RulesNeedUpdateEvent {});
    }
}

fn palette_select(
    mut events: EventReader<PickingEvent>,
    mut selection: ResMut<TileSelection>,
    palette_query: Query<&Tile, With<PaletteTag>>,
    rules: Res<Rules>,
) {
    for event in events.iter() {
        match event {
            PickingEvent::Clicked(e) => {
                match palette_query.get(*e) {
                    Ok(e) => {
                        selection.prototype = Some(rules.prototypes[e.prototype_index].clone())
                    }
                    Err(_) => (),
                };
            }
            _ => (),
        }
    }
}

fn on_mouse_wheel(
    mut mouse_wheel_events: EventReader<MouseWheel>,
    mut selection: ResMut<TileSelection>,
) {
    for event in mouse_wheel_events.iter() {
        selection.rotation += event.y as i32;
    }
}
