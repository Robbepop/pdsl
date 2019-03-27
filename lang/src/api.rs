// Copyright 2018-2019 Parity Technologies (UK) Ltd.
// This file is part of pDSL.
//
// pDSL is free software: you can redistribute it and/or modify
// it under the terms of the GNU General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.
//
// pDSL is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
// GNU General Public License for more details.
//
// You should have received a copy of the GNU General Public License
// along with pDSL.  If not, see <http://www.gnu.org/licenses/>.

use crate::{
    ast,
    errors::{
        Errors,
        Result,
    },
    hir,
    ident_ext::IdentExt,
};
use serde::{
    Deserialize,
    Serialize,
};
use std::convert::TryFrom;

/// Describes a message parameter or return type.
#[derive(Debug, PartialEq, Eq, Deserialize, Serialize)]
pub enum TypeDescription {
    /// The `bool` primitive type.
    #[serde(rename = "bool")]
    Bool,
    /// The `u8` primitive unsigned integer.
    #[serde(rename = "u8")]
    U8,
    /// The `u16` primitive unsigned integer.
    #[serde(rename = "u16")]
    U16,
    /// The `u32` primitive unsigned integer.
    #[serde(rename = "u32")]
    U32,
    /// The `u64` primitive unsigned integer.
    #[serde(rename = "u64")]
    U64,
    /// The `u128` primitive unsigned integer.
    #[serde(rename = "u128")]
    U128,
    /// The `i8` primitive signed integer.
    #[serde(rename = "i8")]
    I8,
    /// The `i16` primitive signed integer.
    #[serde(rename = "i16")]
    I16,
    /// The `i32` primitive signed integer.
    #[serde(rename = "i32")]
    I32,
    /// The `i64` primitive signed integer.
    #[serde(rename = "i64")]
    I64,
    /// The `i128` primitive signed integer.
    #[serde(rename = "i128")]
    I128,
    /// The SRML address type.
    Address,
    /// The SRML balance type.
    Balance,
    /// The tuple type
    Tuple {
        elems: Vec<TypeDescription>,
    },
    /// The fixed size array type
    Array {
        inner: Box<TypeDescription>,
        arity: u32,
    }
}

impl TryFrom<&syn::Type> for TypeDescription {
    type Error = Errors;

    fn try_from(ty: &syn::Type) -> Result<Self> {
        use quote::ToTokens;
        let primitive = |ty: &syn::Type| {
            match ty.into_token_stream().to_string().as_str() {
                "bool" => Ok(TypeDescription::Bool),
                "u8" => Ok(TypeDescription::U8),
                "u16" => Ok(TypeDescription::U16),
                "u32" => Ok(TypeDescription::U32),
                "u64" => Ok(TypeDescription::U64),
                "u128" => Ok(TypeDescription::U128),
                "i8" => Ok(TypeDescription::I8),
                "i16" => Ok(TypeDescription::I16),
                "i32" => Ok(TypeDescription::I32),
                "i64" => Ok(TypeDescription::I64),
                "i128" => Ok(TypeDescription::I128),
                "Address" => Ok(TypeDescription::Address),
                "Balance" => Ok(TypeDescription::Balance),
                unsupported => {
                    bail!(
                        ty,
                        "{} is unsupported as message interface type",
                        unsupported
                    )
                }
            }
        };
        match ty {
            syn::Type::Tuple(tuple) => {
                let elems = tuple
                    .elems
                    .iter()
                    .map(primitive)
                    .collect::<Result<_>>()?;
                Ok(TypeDescription::Tuple { elems })
            },
            syn::Type::Array(array) => {
                let inner = Box::new(primitive(&array.elem)?);
                if let syn::Expr::Lit(syn::ExprLit {
                    lit: syn::Lit::Int(ref int_lit), ..
                }) = array.len {
                    Ok(TypeDescription::Array {
                        inner,
                        arity: int_lit.value() as u32,
                    })
                } else {
                    bail!(
                        array.len,
                        "invalid array length expression"
                    )
                }
            }
            ty => primitive(ty),
        }
    }
}

/// Describes a pair of parameter name and type.
#[derive(Debug, PartialEq, Eq, Deserialize, Serialize)]
pub struct ParamDescription {
    /// The name of the parameter.
    name: String,
    /// The type of the parameter.
    ty: TypeDescription,
}

impl TryFrom<&syn::ArgCaptured> for ParamDescription {
    type Error = Errors;

    fn try_from(arg: &syn::ArgCaptured) -> Result<Self> {
        let name = match &arg.pat {
            syn::Pat::Ident(ident) => ident.ident.to_owned_string(),
            _ => {
                bail!(arg.pat, "unsupported type pattern, currently only identifiers like `foo` are supported")
            }
        };
        Ok(Self {
            name,
            ty: TypeDescription::try_from(&arg.ty)?,
        })
    }
}

/// Describes the deploy handler of a contract.
#[derive(Debug, PartialEq, Eq, Deserialize, Serialize)]
pub struct DeployDescription {
    /// The parameters of the deploy handler.
    params: Vec<ParamDescription>,
}

impl TryFrom<&hir::DeployHandler> for DeployDescription {
    type Error = Errors;

    fn try_from(deploy_handler: &hir::DeployHandler) -> Result<Self> {
        let params = deploy_handler
            .decl
            .inputs
            .iter()
            .filter_map(|arg| {
                match arg {
                    ast::FnArg::Captured(captured) => {
                        let description = ParamDescription::try_from(captured);
                        Some(description)
                    }
                    _ => None,
                }
            })
            .collect::<Result<Vec<_>>>()?;
        Ok(Self { params })
    }
}

/// Describes the return type of a contract message.
#[derive(Debug, PartialEq, Eq, Deserialize, Serialize)]
pub struct ReturnTypeDescription(Option<TypeDescription>);

impl ReturnTypeDescription {
    /// Creates a new return type description from the given optional type.
    pub fn new<T>(opt_type: T) -> Self
    where
        T: Into<Option<TypeDescription>>,
    {
        Self(opt_type.into())
    }
}

impl TryFrom<&syn::ReturnType> for ReturnTypeDescription {
    type Error = Errors;

    fn try_from(ret_ty: &syn::ReturnType) -> Result<Self> {
        match ret_ty {
            syn::ReturnType::Default => Ok(ReturnTypeDescription::new(None)),
            syn::ReturnType::Type(_, ty) => {
                Ok(ReturnTypeDescription::new(Some(TypeDescription::try_from(
                    &**ty,
                )?)))
            }
        }
    }
}

/// Describes a contract message.
#[derive(Debug, PartialEq, Eq, Deserialize, Serialize)]
pub struct MessageDescription {
    /// The name of the message.
    name: String,
    /// The selector hash of the message.
    selector: u64,
    /// If the message is allowed to mutate the contract state.
    mutates: bool,
    /// The parameters of the message.
    params: Vec<ParamDescription>,
    /// The return type of the message.
    ret_ty: ReturnTypeDescription,
}

impl TryFrom<&hir::Message> for MessageDescription {
    type Error = Errors;

    fn try_from(message: &hir::Message) -> Result<Self> {
        Ok(Self {
            name: message.sig.ident.to_owned_string(),
            selector: message.selector().into(),
            mutates: message.is_mut(),
            params: {
                message
                    .sig
                    .decl
                    .inputs
                    .iter()
                    .filter_map(|arg| {
                        match arg {
                            ast::FnArg::Captured(captured) => {
                                Some(ParamDescription::try_from(captured))
                            }
                            _ => None,
                        }
                    })
                    .collect::<Result<Vec<_>>>()?
            },
            ret_ty: ReturnTypeDescription::try_from(&message.sig.decl.output)?,
        })
    }
}

/// Describes a contract.
#[derive(Debug, PartialEq, Eq, Deserialize, Serialize)]
pub struct ContractDescription {
    /// The name of the contract.
    name: String,
    /// The deploy handler of the contract.
    deploy: DeployDescription,
    /// The external messages of the contract.
    messages: Vec<MessageDescription>,
}

impl ContractDescription {
    /// Returns the name of the contract.
    pub fn name(&self) -> &str {
        &self.name
    }
}

impl TryFrom<&hir::Contract> for ContractDescription {
    type Error = Errors;

    fn try_from(contract: &hir::Contract) -> Result<Self> {
        Ok(ContractDescription {
            name: contract.name.to_owned_string(),
            deploy: DeployDescription::try_from(&contract.on_deploy)?,
            messages: {
                contract
                    .messages
                    .iter()
                    .map(MessageDescription::try_from)
                    .collect::<Result<Vec<_>>>()?
            },
        })
    }
}

/// Writes a JSON API description into the `target/` folder.
pub fn generate_api_description(contract: &hir::Contract) -> Result<()> {
    let description = ContractDescription::try_from(contract)?;
    let contents = serde_json::to_string(&description)
        .expect("Failed at generating JSON API description as JSON");
    let mut path_buf = String::from("target/");
    path_buf.push_str(description.name());
    path_buf.push_str(".json");
    std::fs::write(path_buf, contents)
        .expect("Failed at writing JSON API descrition to file");
    Ok(())
}
