use std::ops::{Index, IndexMut};

use serde::{Serialize, Deserialize};


pub type EntityId = u64;

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Comp<T> {
    pub ids: Vec<EntityId>,
    pub store: Vec<T>,
}

impl<T> Comp<T> {
    pub fn new() -> Comp<T> {
        return Comp { ids: Vec::new(), store: Vec::new() };
    }

    pub fn insert(&mut self, entity_id: EntityId, data: T) {
        let result = self.ids.binary_search(&entity_id);
        match result {
            Ok(ok_index) => {
               self.store[ok_index] = data;
            }

            Err(err_index) => {
                self.ids.insert(err_index, entity_id);
                self.store.insert(err_index, data);
            }
        }
    }

    pub fn clear(&mut self) {
        self.ids.clear();
        self.store.clear();
    }

    pub fn remove(&mut self, entity_id: &EntityId) {
        let result = self.ids.binary_search(&entity_id);
        match result {
            Ok(index) => {
                self.ids.remove(index);
                self.store.remove(index);
            }

            Err(_) => {},
        }
    }

    pub fn lookup(&self, entity_id: EntityId) -> usize {
        if let Ok(index) = self.ids.binary_search(&entity_id) {
            return index;
        } else {
            panic!("Component not found for entity!");
        }
    }

    pub fn get(&self, entity_id: &EntityId) -> Option<&T> {
        let result = self.ids.binary_search(entity_id);
        match result {
            Ok(index) => {
                return Some(&self.store[index]);
            }

            Err(_) => {
                return None;
            },
        }
    }

    pub fn get_mut(&mut self, entity_id: &EntityId) -> Option<&mut T> {
        let result = self.ids.binary_search(entity_id);
        match result {
            Ok(index) => {
                return Some(&mut self.store[index]);
            }

            Err(_) => {
                return None;
            },
        }
    }

    pub fn contains_key(&self, entity_id: &EntityId) -> bool {
        return self.ids.binary_search(entity_id).is_ok();
    }

    pub fn iter(&self) -> CompIter<'_, T> {
        return CompIter::new(self);
    }
}

impl<T> Default for Comp<T> {
    fn default() -> Comp<T> {
        return Comp::new();
    }
}

impl<T> Index<&EntityId> for Comp<T> {
    type Output = T;

    fn index(&self, index: &EntityId) -> &T {
        let store_index = self.lookup(*index);
        return &self.store[store_index];
    }
}

impl<T> IndexMut<&EntityId> for Comp<T> {
    fn index_mut(&mut self, index: &EntityId) -> &mut T {
        let store_index = self.lookup(*index);
        return &mut self.store[store_index];
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct CompIter<'a, T> {
    comp: &'a Comp<T>,
    index: usize,
}

impl<'a, T> CompIter<'a, T> {
    pub fn new(comp: &'a Comp<T>) -> CompIter<'a, T> {
        return CompIter { comp, index: 0 };
    }
}

impl<'a, T> Iterator for CompIter<'a, T> {
    type Item = (u64, &'a T);

    fn next(&mut self) -> Option<Self::Item> {
        if self.index >= self.comp.ids.len() {
            return None;
        } else {
            let index = self.index;
            self.index += 1;
            return Some((self.comp.ids[index], &self.comp.store[index]));
        }
    }
}

