use rustdoc_types::{GenericArg, GenericArgs, GenericParamDefKind, ItemEnum, Type};

pub(crate) fn render_item_signature(name: Option<&str>, item: &ItemEnum) -> Option<String> {
    let name = name?;
    match item {
        ItemEnum::Function(function) => Some(render_function(name, function)),
        ItemEnum::Struct(_) => Some(format!("pub struct {name}")),
        ItemEnum::Enum(_) => Some(format!("pub enum {name}")),
        ItemEnum::Union(_) => Some(format!("pub union {name}")),
        ItemEnum::Trait(_) => Some(format!("pub trait {name}")),
        ItemEnum::TypeAlias(alias) => {
            Some(format!("pub type {name} = {}", render_type(&alias.type_)))
        }
        _ => None,
    }
}

fn render_function(name: &str, function: &rustdoc_types::Function) -> String {
    let mut out = String::from("pub ");
    if function.header.is_const {
        out.push_str("const ");
    }
    if function.header.is_async {
        out.push_str("async ");
    }
    if function.header.is_unsafe {
        out.push_str("unsafe ");
    }
    out.push_str("fn ");
    out.push_str(name);

    let params: Vec<_> = function
        .generics
        .params
        .iter()
        .filter_map(|param| match &param.kind {
            GenericParamDefKind::Type {
                is_synthetic: true, ..
            } => None,
            GenericParamDefKind::Const { type_, .. } => {
                Some(format!("const {}: {}", param.name, render_type(type_)))
            }
            _ => Some(param.name.clone()),
        })
        .collect();
    if !params.is_empty() {
        out.push('<');
        out.push_str(&params.join(", "));
        out.push('>');
    }

    out.push('(');
    out.push_str(
        &function
            .sig
            .inputs
            .iter()
            .map(|(name, ty)| format!("{name}: {}", render_type(ty)))
            .collect::<Vec<_>>()
            .join(", "),
    );
    if function.sig.is_c_variadic {
        if !function.sig.inputs.is_empty() {
            out.push_str(", ");
        }
        out.push_str("...");
    }
    out.push(')');
    if let Some(output) = &function.sig.output {
        out.push_str(" -> ");
        out.push_str(&render_type(output));
    }
    out
}

fn render_type(ty: &Type) -> String {
    match ty {
        Type::ResolvedPath(path) => {
            let mut out = path.path.clone();
            if let Some(args) = &path.args {
                out.push_str(&render_args(args));
            }
            out
        }
        Type::Generic(name) | Type::Primitive(name) => name.clone(),
        Type::Tuple(types) => {
            let mut rendered = types.iter().map(render_type).collect::<Vec<_>>().join(", ");
            if types.len() == 1 {
                rendered.push(',');
            }
            format!("({rendered})")
        }
        Type::Slice(type_) => format!("[{}]", render_type(type_)),
        Type::Array { type_, len } => format!("[{}; {len}]", render_type(type_)),
        Type::RawPointer { is_mutable, type_ } => format!(
            "*{} {}",
            if *is_mutable { "mut" } else { "const" },
            render_type(type_)
        ),
        Type::BorrowedRef {
            lifetime,
            is_mutable,
            type_,
        } => format!(
            "&{}{}{}",
            lifetime
                .as_deref()
                .map(|value| format!("{value} "))
                .unwrap_or_default(),
            if *is_mutable { "mut " } else { "" },
            render_type(type_)
        ),
        Type::ImplTrait(_) => "impl Trait".into(),
        Type::DynTrait(_) => "dyn Trait".into(),
        Type::FunctionPointer(function) => {
            let inputs = function
                .sig
                .inputs
                .iter()
                .map(|(_, ty)| render_type(ty))
                .collect::<Vec<_>>()
                .join(", ");
            let output = function
                .sig
                .output
                .as_ref()
                .map(|ty| format!(" -> {}", render_type(ty)))
                .unwrap_or_default();
            format!("fn({inputs}){output}")
        }
        Type::QualifiedPath {
            name, self_type, ..
        } => format!("{}::{name}", render_type(self_type)),
        Type::Infer => "_".into(),
        Type::Pat { type_, .. } => render_type(type_),
    }
}

fn render_args(args: &GenericArgs) -> String {
    match args {
        GenericArgs::AngleBracketed { args, .. } => {
            let rendered = args
                .iter()
                .map(|arg| match arg {
                    GenericArg::Lifetime(value) => value.clone(),
                    GenericArg::Type(ty) => render_type(ty),
                    GenericArg::Const(value) => value.expr.clone(),
                    GenericArg::Infer => "_".into(),
                })
                .collect::<Vec<_>>()
                .join(", ");
            format!("<{rendered}>")
        }
        GenericArgs::Parenthesized { inputs, output } => {
            let inputs = inputs
                .iter()
                .map(render_type)
                .collect::<Vec<_>>()
                .join(", ");
            let output = output
                .as_ref()
                .map(|ty| format!(" -> {}", render_type(ty)))
                .unwrap_or_default();
            format!("({inputs}){output}")
        }
        GenericArgs::ReturnTypeNotation => "(..)".into(),
    }
}
