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

use crate::ir;
use core::convert::TryFrom;

#[test]
fn is_ink_impl_block_eval_false_works() {
    let item_impls: Vec<syn::ItemImpl> = vec![
        syn::parse_quote! {
            impl MyStorage {}
        },
        syn::parse_quote! {
            impl MyTrait for MyStorage {}
        },
    ];
    for item_impl in &item_impls {
        assert_eq!(
            ir::ItemImpl::is_ink_impl_block(item_impl).map_err(|err| err.to_string()),
            Ok(false),
        )
    }
}

#[test]
fn is_ink_impl_block_eval_true_works() {
    let item_impls: Vec<syn::ItemImpl> = vec![
        syn::parse_quote! {
            #[ink(impl)]
            impl MyStorage {}
        },
        syn::parse_quote! {
            impl MyStorage {
                #[ink(constructor)]
                fn my_constructor() -> Self {}
            }
        },
        syn::parse_quote! {
            impl MyStorage {
                #[ink(message)]
                fn my_message(&self) {}
            }
        },
        syn::parse_quote! {
            #[ink(impl)]
            impl MyTrait for MyStorage {}
        },
        syn::parse_quote! {
            impl MyTrait for MyStorage {
                #[ink(message)]
                fn my_message(&self) {}
            }
        },
        syn::parse_quote! {
            #[ink(impl)]
            impl MyStorage {
                #[ink(constructor)]
                fn my_constructor() -> Self {}
                #[ink(message)]
                fn my_message(&self) {}
            }
        },
        syn::parse_quote! {
            #[ink(impl)]
            impl MyTrait for MyStorage {
                #[ink(constructor)]
                fn my_constructor() -> Self {}
                #[ink(message)]
                fn my_message(&self) {}
            }
        },
        syn::parse_quote! {
            // This is actually invalid but the function under test will
            // still determine this to be a valid ink! implementation block.
            #[ink(impl)]
            impl MyStorage {
                #[ink(..)]
                fn invalid_ink_attribute(&self) {}
            }
        },
    ];
    for item_impl in &item_impls {
        assert_eq!(
            ir::ItemImpl::is_ink_impl_block(item_impl).map_err(|err| err.to_string()),
            Ok(true),
        )
    }
}

fn assert_is_ink_impl_block_fails(impl_block: &syn::ItemImpl, expected: &str) {
    assert_eq!(
        ir::ItemImpl::is_ink_impl_block(impl_block).map_err(|err| err.to_string()),
        Err(expected.to_string())
    )
}

#[test]
fn is_ink_impl_block_fails() {
    assert_is_ink_impl_block_fails(
        &syn::parse_quote! {
            #[ink(invalid)]
            impl MyStorage {}
        },
        "unknown ink! attribute (path)",
    );
    assert_is_ink_impl_block_fails(
        &syn::parse_quote! {
            #[ink(invalid)]
            impl MyTrait for MyStorage {}
        },
        "unknown ink! attribute (path)",
    );
    assert_is_ink_impl_block_fails(
        &syn::parse_quote! {
            #[ink(impl)]
            #[ink(impl)]
            impl MyStorage {}
        },
        "encountered duplicate ink! attribute",
    );
    assert_is_ink_impl_block_fails(
        &syn::parse_quote! {
            #[ink(impl)]
            #[ink(impl)]
            impl MyTrait for MyStorage {}
        },
        "encountered duplicate ink! attribute",
    );
    assert_is_ink_impl_block_fails(
        &syn::parse_quote! {
            impl MyStorage {
                #[ink(invalid)]
                fn invalid_fn_attr(&self) {}
            }
        },
        "unknown ink! attribute (path)",
    );
    assert_is_ink_impl_block_fails(
        &syn::parse_quote! {
            impl MyTrait for MyStorage {
                #[ink(invalid)]
                fn invalid_fn_attr(&self) {}
            }
        },
        "unknown ink! attribute (path)",
    );
}

/// Asserts that the `TryFrom` application on the given [`syn::ItemImpl`]
/// fails with the expected error message.
fn assert_try_from_item_impl_fails(item_impl: syn::ItemImpl, expected_err: &str) {
    assert_eq!(
        <ir::ItemImpl as TryFrom<syn::ItemImpl>>::try_from(item_impl)
            .map_err(|err| err.to_string()),
        Err(expected_err.to_string())
    )
}

#[test]
fn visibility_fails() {
    assert_try_from_item_impl_fails(
        syn::parse_quote! {
            impl MyStorage {
                #[ink(message)]
                fn my_private_message(&self) {}
            }
        },
        "ink! message in inherent impl blocks must have public visibility",
    );
    assert_try_from_item_impl_fails(
        syn::parse_quote! {
            impl MyStorage {
                #[ink(constructor)]
                fn my_private_constructor() -> Self {}
            }
        },
        "ink! constructor in inherent impl blocks must have public visibility",
    );
    assert_try_from_item_impl_fails(
        syn::parse_quote! {
            impl MyTrait for MyStorage {
                #[ink(message)]
                pub fn my_public_message(&self) {}
            }
        },
        "ink! message in trait impl blocks must have inherited visibility",
    );
    assert_try_from_item_impl_fails(
        syn::parse_quote! {
            impl MyTrait for MyStorage {
                #[ink(constructor)]
                pub fn my_public_constructor() -> Self {}
            }
        },
        "ink! constructor in trait impl blocks must have inherited visibility",
    );
}

#[test]
fn try_from_works() {
    let item_impls: Vec<syn::ItemImpl> = vec![
        syn::parse_quote! {
            #[ink(impl)]
            impl MyStorage {}
        },
        syn::parse_quote! {
            impl MyStorage {
                #[ink(message)]
                pub fn my_message(&self) {}
            }
        },
        syn::parse_quote! {
            #[ink(impl)]
            impl MyTrait for MyStorage {}
        },
        syn::parse_quote! {
            impl MyTrait for MyStorage {
                #[ink(message)]
                fn my_message(&self) {}
            }
        },
    ];
    for item_impl in item_impls {
        assert!(<ir::ItemImpl as TryFrom<syn::ItemImpl>>::try_from(item_impl).is_ok())
    }
}

#[test]
fn namespace_works() {
    let impl_block: ir::ItemImpl =
        <ir::ItemImpl as TryFrom<syn::ItemImpl>>::try_from(syn::parse_quote! {
            #[ink(namespace = "my_namespace")]
            impl MyStorage {
                #[ink(message)]
                pub fn my_message(&self) {}
            }
        })
        .unwrap();
    assert_eq!(
        impl_block.namespace(),
        Some(&ir::Namespace::from(
            "my_namespace".to_string().as_bytes().to_vec()
        ))
    )
}

#[test]
fn metadata_name_default_works() {
    let impl_block: ir::ItemImpl =
        <ir::ItemImpl as TryFrom<syn::ItemImpl>>::try_from(syn::parse_quote! {
            impl Storage for MyStorage {
                #[ink(message)]
                fn my_message(&self) {}
            }
        })
        .unwrap();
    assert_eq!(
        impl_block.trait_metadata_name(),
        Some(String::from("Storage"))
    )
}

#[test]
fn metadata_name_works() {
    let impl_block: ir::ItemImpl =
        <ir::ItemImpl as TryFrom<syn::ItemImpl>>::try_from(syn::parse_quote! {
            #[ink(metadata_name = "Storage2")]
            impl Storage for MyStorage {
                #[ink(message)]
                fn my_message(&self) {}
            }
        })
        .unwrap();
    assert_eq!(
        impl_block.trait_metadata_name(),
        Some(String::from("Storage2"))
    )
}

#[test]
fn metadata_name_empty_works() {
    let impl_block: ir::ItemImpl =
        <ir::ItemImpl as TryFrom<syn::ItemImpl>>::try_from(syn::parse_quote! {
            #[ink(metadata_name = "")]
            impl Storage for MyStorage {
                #[ink(message)]
                fn my_message(&self) {}
            }
        })
        .unwrap();
    assert_eq!(impl_block.trait_metadata_name(), None)
}
