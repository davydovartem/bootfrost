use serde::{Deserialize, Serialize};

use std::ops::Index;
use std::collections::HashMap;

use crate::misc::*;
use crate::term::*;

use crate::strategies::attributes::*;


#[derive(Debug, Deserialize, Serialize)]
pub struct Base{
	pub base: Vec<BTerm>,
	index: HashMap<TermId, BaseAtomId>,
	by_id: HashMap<BaseAtomId, usize>,
	next_id: usize,
}

impl Base{
	pub fn new() -> Base{
		Base{
			base: vec![],
			index: HashMap::new(),
			by_id: HashMap::new(),
			next_id: 0,
		}
	}

	pub fn len(&self) -> usize{
		self.base.len()
	}

	pub fn is_empty(&self) -> bool{
		self.base.is_empty()
	}

	pub fn get_mut(&mut self, i:usize) -> Option<&mut BTerm>{
		self.base.get_mut(i)
	}

	fn rebuild_indexes(&mut self){
		self.index.clear();
		self.by_id.clear();

		for (i, bt) in self.base.iter().enumerate(){
			self.by_id.insert(bt.id, i);
			self.index.insert(bt.term, bt.id);
		}
	}

	pub fn push(&mut self, tid:TermId, bid: BlockId){
		let id = BaseAtomId(self.next_id);
		self.next_id += 1;

		let position = self.base.len();
		self.index.insert(tid, id);
		self.by_id.insert(id, position);
		self.base.push(BTerm{id: id, term: tid, bid: bid, deleted: false})
	}

	pub fn latest_id_for_term(&self, tid: &TermId) -> Option<BaseAtomId>{
		self.index.get(tid).copied()
	}

	pub fn contains_live_term(&self, tid: &TermId, attributes: &Attributes) -> bool{
		if let Some(id) = self.latest_id_for_term(tid){
			if let Some(bt) = self.get_by_id(id){
				!attributes.check(KeyObject::BaseAtom(bt.id), AttributeName("deleted".to_string()), AttributeValue("true".to_string()))
			}else{
				false
			}
		}else{
			false
		}
	}

	pub fn push_and_check(&mut self, tid:TermId, bid:BlockId, attributes: &Attributes) -> bool{
		if self.contains_live_term(&tid, attributes){
			false
		}else{
			self.push(tid, bid);
			true
		}
	}

	pub fn remove(&mut self, bid:BlockId){
		let mut removed = false;
		while let Some(last) = self.base.last(){
			if last.bid == bid{
				if let Some(bt) = self.base.pop(){
					self.by_id.remove(&bt.id);
					removed = true;
				}else{
					panic!("");
				}
			}else{
				break;
			}
		}

		if removed{
			self.rebuild_indexes();
		}
	}

	pub fn deleted(&self, i:usize) -> bool{
		self.base[i].deleted
	}

	pub fn contains_key(&self, tid: &TermId) -> bool{
		self.index.contains_key(tid)
	}

	pub fn get_by_id(&self, id: BaseAtomId) -> Option<&BTerm>{
		self.by_id.get(&id).and_then(|i| self.base.get(*i))
	}

	pub fn term_by_id(&self, id: BaseAtomId) -> Option<TermId>{
		self.get_by_id(id).map(|bt| bt.term)
	}

	pub fn position_of(&self, id: BaseAtomId) -> Option<usize>{
		self.by_id.get(&id).copied()
	}

	pub fn contains_id(&self, id: BaseAtomId) -> bool{
		self.by_id.contains_key(&id)
	}

	pub fn all_ids(&self) -> Vec<BaseAtomId>{
		self.base.iter().map(|bt| bt.id).collect()
	}


}

impl Index<usize> for Base{
	type Output = BTerm;

	fn index (&self, i:usize) -> &Self::Output{
		&self.base[i]
	}
}
