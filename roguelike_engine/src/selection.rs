use serde::{Serialize, Deserialize};

use roguelike_core::types::*;
use roguelike_core::utils::*;
use roguelike_core::movement::{Direction, Action, Reach};
use roguelike_core::map::*;


#[derive(Copy, Clone, PartialEq, Debug, Serialize, Deserialize)]
pub enum SelectionAction {
    Throw,
    Hammer,
    Interact,
    PlaceTrap,
    GrassThrow,
    PassWall,
    Rubble,
    Reform,
    Swap,
    Push,
}

impl SelectionAction {
    pub fn action_from_pos(&self, pos: Pos, data: &GameData) -> Action {
        let mut action: Action;

        match self {
            SelectionAction::Throw => {
                let player_id = data.find_by_name(EntityName::Player).unwrap();
                let item_id =
                    data.entities.selected_item.get(&player_id).expect("Throwing an item, but nothing selected!");
                action = Action::ThrowItem(pos, *item_id);
            }

            SelectionAction::Hammer => {
                action = Action::UseItem(pos);
            }

            SelectionAction::Interact => {
                action = Action::NoAction;
                for entity_id  in data.has_entity(pos) {
                    if let Some(_trap) = data.entities.trap.get(&entity_id) {
                        action = Action::ArmDisarmTrap(entity_id);
                        break;
                    }
                }
            }

            SelectionAction::PlaceTrap => {
                let player_id = data.find_by_name(EntityName::Player).unwrap();
                let trap_id =
                    data.entities.selected_item.get(&player_id).expect("Placing a trap, but nothing selected!");
                action = Action::PlaceTrap(pos, *trap_id);
            }

            SelectionAction::GrassThrow => {
                let player_id = data.find_by_name(EntityName::Player).unwrap();
                let player_pos = data.entities.pos[&player_id];
                let dxy = sub_pos(pos, player_pos);
                let direction = Direction::from_dxy(dxy.x, dxy.y).unwrap();
                action = Action::GrassThrow(player_id, direction);
            }

            SelectionAction::PassWall => {
                action = Action::NoAction;

                let player_id = data.find_by_name(EntityName::Player).unwrap();
                let player_pos = data.entities.pos[&player_id];
                let blocked = data.map.path_blocked_move(player_pos, pos);
                
                if let Some(blocked) = blocked {
                    if data.map[blocked.end_pos].block_move {
                        let next = next_pos(player_pos, blocked.end_pos);
                        if  !data.map[next].block_move {
                            action = Action::PassWall(player_id, next);
                        }
                    } else {
                        action = Action::PassWall(player_id, pos);
                    }
                }
            }

            SelectionAction::Rubble => {
                action = Action::NoAction;

                let player_id = data.find_by_name(EntityName::Player).unwrap();
                let player_pos = data.entities.pos[&player_id];
                let blocked = data.map.path_blocked_move(player_pos, pos);

                if let Some(blocked) = blocked {
                    if data.has_blocking_entity(pos).is_none() {
                        action = Action::Rubble(player_id, blocked);
                    }
                }
            }

            SelectionAction::Reform => {
                action = Action::NoAction;

                let player_id = data.find_by_name(EntityName::Player).unwrap();
                if data.map[pos].surface == Surface::Rubble &&
                   data.has_blocking_entity(pos).is_none() {
                    action = Action::Reform(player_id, pos);
                }
            }

            SelectionAction::Swap => {
                action = Action::NoAction;

                let player_id = data.find_by_name(EntityName::Player).unwrap();
                if let Some(entity_id) = data.has_blocking_entity(pos) {
                    action = Action::Swap(player_id, entity_id);
                }
            }

            SelectionAction::Push => {
                let player_id = data.find_by_name(EntityName::Player).unwrap();
                let player_pos = data.entities.pos[&player_id];
                let dxy = sub_pos(pos, player_pos);
                let direction = Direction::from_dxy(dxy.x, dxy.y).unwrap();
                action = Action::Push(player_id, direction);
            }
        }

        return action;
    }
}


#[derive(Copy, Clone, PartialEq, Debug, Serialize, Deserialize)]
pub enum SelectionType {
    WithinReach(Reach),
    WithinRadius(usize),
}

impl SelectionType {
    pub fn offset_pos(&self, pos: Pos, dir: Direction) -> Option<Pos> {
        match self {
            SelectionType::WithinReach(reach) => {
                return reach.furthest_in_direction(pos, dir);
            }

            SelectionType::WithinRadius(dist) => {
                return Some(dir.offset_pos(pos, *dist as i32));
            }
        }
    }
}

#[derive(Copy, Clone, PartialEq, Debug, Serialize, Deserialize)]
pub struct Selection {
    pub typ: SelectionType,
    pub action: SelectionAction,
    pub only_visible: bool,
    // TODO consider adding:
    // SelectionFilter enum with Entity/Wall/Empty/Any
    // position to selection will have to check available positions and find one that matches
    // the filter
}

impl Default for Selection {
    fn default() -> Selection {
        return Selection::new(SelectionType::WithinRadius(0), SelectionAction::Throw);
    }
}

impl Selection {
    pub fn new(typ: SelectionType, action: SelectionAction) -> Self {
        return Selection {
            typ,
            action,
            only_visible: false,
        };
    }

    pub fn selected_pos(&self, pos: Pos, selected: Pos, fov_radius: i32, data: &GameData) -> Option<Pos> {
        let mut maybe_selected_pos: Option<Pos>;

        match self.typ {
            SelectionType::WithinReach(reach) => {
                let selected_pos = reach.closest_to(pos, selected);

                maybe_selected_pos = Some(selected_pos);
            }

            SelectionType::WithinRadius(radius) => {
                let selected_pos: Pos;
                if distance(selected, pos) as usize <= radius {
                    selected_pos = selected;
                } else {
                    selected_pos = move_towards(pos, selected, radius);
                }

                maybe_selected_pos = Some(selected_pos);
            }
        }

        if self.only_visible {
            if let Some(selected_pos) = maybe_selected_pos {
                if !data.map.is_in_fov(pos, selected_pos, fov_radius) {
                    maybe_selected_pos = None;
                }
            }
        }

        return maybe_selected_pos;
    }

    pub fn select(&self, pos: Pos, selected: Pos, fov_radius: i32, data: &GameData) -> Option<Action> {
        let maybe_selected_pos: Option<Pos> = self.selected_pos(pos, selected, fov_radius, data);

        if let Some(selected_pos) = maybe_selected_pos {
            return Some(self.action.action_from_pos(selected_pos, data));
        } else {
            return None;
        }
    }
}

