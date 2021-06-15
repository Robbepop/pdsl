// Copyright 2018-2021 Parity Technologies (UK) Ltd.
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
//     http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

use crate::{
    ir,
    InkTrait,
    InkTraitConstructor,
    InkTraitItem,
    InkTraitMessage,
    Selector,
};
use std::collections::HashMap;

/// Iterator over all the ink! trait items of an ink! trait definition.
pub struct IterInkTraitItemsRaw<'a> {
    iter: core::slice::Iter<'a, syn::TraitItem>,
}

impl<'a> IterInkTraitItemsRaw<'a> {
    /// Creates a new iterator yielding ink! trait items over the raw Rust trait definition.
    pub(super) fn from_raw(item_trait: &'a syn::ItemTrait) -> Self {
        Self {
            iter: item_trait.items.iter(),
        }
    }
}

impl<'a> Iterator for IterInkTraitItemsRaw<'a> {
    type Item = InkTraitItem<'a>;

    fn next(&mut self) -> Option<Self::Item> {
        'outer: loop {
            match self.iter.next() {
                None => return None,
                Some(syn::TraitItem::Method(method)) => {
                    let first_attr = ir::first_ink_attribute(&method.attrs)
                        .ok()
                        .flatten()
                        .expect("unexpected missing ink! attribute for trait method")
                        .first()
                        .kind()
                        .clone();
                    match first_attr {
                        ir::AttributeArg::Constructor => {
                            return Some(InkTraitItem::Constructor(
                                InkTraitConstructor::new(method),
                            ))
                        }
                        ir::AttributeArg::Message => {
                            return Some(InkTraitItem::Message(InkTraitMessage::new(
                                method,
                            )))
                        }
                        _ => continue 'outer,
                    }
                }
                Some(_) => continue 'outer,
            }
        }
    }
}

/// Iterator over all the ink! trait items of an ink! trait definition.
pub struct IterInkTraitItems<'a> {
    iter: IterInkTraitItemsRaw<'a>,
    message_selectors: &'a HashMap<syn::Ident, Selector>,
    constructor_selectors: &'a HashMap<syn::Ident, Selector>,
}

impl<'a> IterInkTraitItems<'a> {
    /// Creates a new iterator yielding ink! trait items.
    pub(super) fn new(item_trait: &'a InkTrait) -> Self {
        Self {
            iter: IterInkTraitItemsRaw::from_raw(&item_trait.item),
            message_selectors: &item_trait.message_selectors,
            constructor_selectors: &item_trait.constructor_selectors,
        }
    }
}

impl<'a> Iterator for IterInkTraitItems<'a> {
    type Item = (InkTraitItem<'a>, Selector);

    fn next(&mut self) -> Option<Self::Item> {
        match self.iter.next() {
            None => None,
            Some(item @ InkTraitItem::Constructor(_)) => {
                let selector = self.constructor_selectors[item.ident()];
                Some((item, selector))
            }
            Some(item @ InkTraitItem::Message(_)) => {
                let selector = self.message_selectors[item.ident()];
                Some((item, selector))
            }
        }
    }
}
