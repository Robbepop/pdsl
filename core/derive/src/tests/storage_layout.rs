// Copyright 2018-2020 Parity Technologies (UK) Ltd.
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

use crate::storage_layout_derive;

#[test]
fn unit_struct_works() {
    synstructure::test_derive! {
        storage_layout_derive {
            struct UnitStruct;
        }
        expands to {
            #[allow(non_upper_case_globals)]
            const _DERIVE_ink_core_storage2_traits_StorageLayout_FOR_UnitStruct: () = {
                impl ::ink_core::storage2::traits::StorageLayout for UnitStruct {
                    fn layout(__key_ptr: &mut ::ink_core::storage2::traits::KeyPtr) -> ::ink_abi::layout2::Layout {
                        ::ink_abi::layout2::Layout::Struct(
                            ::ink_abi::layout2::StructLayout::new(vec![])
                        )
                    }
                }
            };
        }
    }
}

#[test]
fn tuple_struct_works() {
    synstructure::test_derive! {
        storage_layout_derive {
            struct TupleStruct(bool, u32, i64);
        }
        expands to {
            #[allow(non_upper_case_globals)]
            const _DERIVE_ink_core_storage2_traits_StorageLayout_FOR_TupleStruct: () = {
                impl ::ink_core::storage2::traits::StorageLayout for TupleStruct {
                    fn layout(__key_ptr: &mut ::ink_core::storage2::traits::KeyPtr) -> ::ink_abi::layout2::Layout {
                        ::ink_abi::layout2::Layout::Struct(
                            ::ink_abi::layout2::StructLayout::new(vec![
                                ::ink_abi::layout2::FieldLayout::new(
                                    None,
                                    <bool as ::ink_core::storage2::traits::StorageLayout>::layout(__key_ptr),
                                ),
                                ::ink_abi::layout2::FieldLayout::new(
                                    None,
                                    <u32 as ::ink_core::storage2::traits::StorageLayout>::layout(__key_ptr),
                                ),
                                ::ink_abi::layout2::FieldLayout::new(
                                    None,
                                    <i64 as ::ink_core::storage2::traits::StorageLayout>::layout(__key_ptr),
                                ),
                            ])
                        )
                    }
                }
            };
        }
    }
}

#[test]
fn named_fields_struct_works() {
    synstructure::test_derive! {
        storage_layout_derive {
            struct NamedFieldsStruct {
                a: bool,
                b: u32,
                c: i64,
            }
        }
        expands to {
            #[allow(non_upper_case_globals)]
            const _DERIVE_ink_core_storage2_traits_StorageLayout_FOR_NamedFieldsStruct: () = {
                impl ::ink_core::storage2::traits::StorageLayout for NamedFieldsStruct {
                    fn layout(__key_ptr: &mut ::ink_core::storage2::traits::KeyPtr) -> ::ink_abi::layout2::Layout {
                        ::ink_abi::layout2::Layout::Struct(
                            ::ink_abi::layout2::StructLayout::new(vec![
                                ::ink_abi::layout2::FieldLayout::new(
                                    Some("a"),
                                    <bool as ::ink_core::storage2::traits::StorageLayout>::layout(__key_ptr),
                                ),
                                ::ink_abi::layout2::FieldLayout::new(
                                    Some("b"),
                                    <u32 as ::ink_core::storage2::traits::StorageLayout>::layout(__key_ptr),
                                ),
                                ::ink_abi::layout2::FieldLayout::new(
                                    Some("c"),
                                    <i64 as ::ink_core::storage2::traits::StorageLayout>::layout(__key_ptr),
                                ),
                            ])
                        )
                    }
                }
            };
        }
    }
}

#[test]
fn clike_enum_works() {
    synstructure::test_derive! {
        storage_layout_derive {
            enum ClikeEnum { A, B, C }
        }
        expands to {
            #[allow(non_upper_case_globals)]
            const _DERIVE_ink_core_storage2_traits_StorageLayout_FOR_ClikeEnum: () = {
                impl ::ink_core::storage2::traits::StorageLayout for ClikeEnum {
                    fn layout(__key_ptr: &mut ::ink_core::storage2::traits::KeyPtr) -> ::ink_abi::layout2::Layout {
                        let dispatch_key = __key_ptr.advance_by(1);
                        ::ink_abi::layout2::Layout::Enum(
                            ::ink_abi::layout2::EnumLayout::new(
                                ::ink_abi::layout2::LayoutKey::from(dispatch_key),
                                vec![
                                    {
                                        let mut __variant_key_ptr = __key_ptr.clone();
                                        let mut __key_ptr = &mut __variant_key_ptr;
                                        (
                                            ::ink_abi::layout2::Discriminant::from(0usize),
                                            ::ink_abi::layout2::StructLayout::new(vec![]),
                                        )
                                    },
                                    {
                                        let mut __variant_key_ptr = __key_ptr.clone();
                                        let mut __key_ptr = &mut __variant_key_ptr;
                                        (
                                            ::ink_abi::layout2::Discriminant::from(1usize),
                                            ::ink_abi::layout2::StructLayout::new(vec![]),
                                        )
                                    },
                                    {
                                        let mut __variant_key_ptr = __key_ptr.clone();
                                        let mut __key_ptr = &mut __variant_key_ptr;
                                        (
                                            ::ink_abi::layout2::Discriminant::from(2usize),
                                            ::ink_abi::layout2::StructLayout::new(vec![]),
                                        )
                                    },
                                ]
                            )
                        )
                    }
                }
            };
        }
    }
}

#[test]
fn mixed_enum_works() {
    synstructure::test_derive! {
        storage_layout_derive {
            enum MixedEnum {
                A,
                B(bool, u32, i64),
                C{
                    a: bool,
                    b: u32,
                    c: i64,
                }
            }
        }
        expands to {
            #[allow(non_upper_case_globals)]
            const _DERIVE_ink_core_storage2_traits_StorageLayout_FOR_MixedEnum: () = {
                impl ::ink_core::storage2::traits::StorageLayout for MixedEnum {
                    fn layout(__key_ptr: &mut ::ink_core::storage2::traits::KeyPtr) -> ::ink_abi::layout2::Layout {
                        let dispatch_key = __key_ptr.advance_by(1);
                        ::ink_abi::layout2::Layout::Enum(
                            ::ink_abi::layout2::EnumLayout::new(
                                ::ink_abi::layout2::LayoutKey::from(dispatch_key),
                                vec![
                                    {
                                        let mut __variant_key_ptr = __key_ptr.clone();
                                        let mut __key_ptr = &mut __variant_key_ptr;
                                        (
                                            ::ink_abi::layout2::Discriminant::from(0usize),
                                            ::ink_abi::layout2::StructLayout::new(vec![]),
                                        )
                                    },
                                    {
                                        let mut __variant_key_ptr = __key_ptr.clone();
                                        let mut __key_ptr = &mut __variant_key_ptr;
                                        (
                                            ::ink_abi::layout2::Discriminant::from(1usize),
                                            ::ink_abi::layout2::StructLayout::new(vec![
                                                ::ink_abi::layout2::FieldLayout::new(
                                                    None,
                                                    <bool as ::ink_core::storage2::traits::StorageLayout>::layout(__key_ptr),
                                                ),
                                                ::ink_abi::layout2::FieldLayout::new(
                                                    None,
                                                    <u32 as ::ink_core::storage2::traits::StorageLayout>::layout(__key_ptr),
                                                ),
                                                ::ink_abi::layout2::FieldLayout::new(
                                                    None,
                                                    <i64 as ::ink_core::storage2::traits::StorageLayout>::layout(__key_ptr),
                                                ),
                                            ]),
                                        )
                                    },
                                    {
                                        let mut __variant_key_ptr = __key_ptr.clone();
                                        let mut __key_ptr = &mut __variant_key_ptr;
                                        (
                                            ::ink_abi::layout2::Discriminant::from(2usize),
                                            ::ink_abi::layout2::StructLayout::new(vec![
                                                ::ink_abi::layout2::FieldLayout::new(
                                                    Some("a"),
                                                    <bool as ::ink_core::storage2::traits::StorageLayout>::layout(__key_ptr),
                                                ),
                                                ::ink_abi::layout2::FieldLayout::new(
                                                    Some("b"),
                                                    <u32 as ::ink_core::storage2::traits::StorageLayout>::layout(__key_ptr),
                                                ),
                                                ::ink_abi::layout2::FieldLayout::new(
                                                    Some("c"),
                                                    <i64 as ::ink_core::storage2::traits::StorageLayout>::layout(__key_ptr),
                                                ),
                                            ]),
                                        )
                                    },
                                ]
                            )
                        )
                    }
                }
            };
        }
    }
}
