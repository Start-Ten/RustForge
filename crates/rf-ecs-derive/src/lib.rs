//! rf-ecs 派生宏：`#[derive(Component)]` / `#[derive(Resource)]`。
//! 零依赖：手工扫描 TokenStream 中的类型名。

use proc_macro::TokenStream;

/// 找到 `struct|enum|union` 之后的类型名（跳过 pub 等修饰不必要——derive 输入
/// 以 `struct Name` 开头段）。返回带路径限定的实现，支持泛型参数原样传递。
fn type_name(input: TokenStream) -> String {
    let tokens: Vec<String> = input.into_iter().map(|t| t.to_string()).collect();
    for (i, t) in tokens.iter().enumerate() {
        if t == "struct" || t == "enum" || t == "union" {
            if let Some(name) = tokens.get(i + 1) {
                return name.clone();
            }
        }
    }
    panic!("derive(Component/Resource): 无法从输入解析类型名");
}

/// 泛型参数提取：Name<T, U> 中的 `<'a, T, U>` 段与 where 子句透传。
fn generics_of(input: TokenStream) -> (String, String, Vec<String>) {
    let tokens: Vec<String> = input.into_iter().map(|t| t.to_string()).collect();
    // 找到 name 后的 <...>（若有）
    let mut name_idx = None;
    for (i, t) in tokens.iter().enumerate() {
        if t == "struct" || t == "enum" || t == "union" {
            name_idx = Some(i + 1);
            break;
        }
    }
    let Some(ni) = name_idx else { return Default::default() };
    let mut decl = String::new();
    let mut params: Vec<String> = Vec::new();
    if tokens.get(ni + 1).map(|s| s.as_str()) == Some("<") {
        let mut depth = 0;
        let mut cur = String::new();
        for t in tokens.iter().skip(ni + 2) {
            if t == "<" {
                depth += 1;
                cur.push('<');
            } else if t == ">" && depth == 0 {
                decl = format!("<{}>", cur.trim().trim_end_matches(',').trim());
                cur.clear();
                break;
            } else if t == ">" {
                depth -= 1;
                cur.push('>');
            } else if t == "," && depth == 0 {
                params.push(cur.trim().to_string());
                cur.clear();
            } else {
                cur.push_str(t);
                cur.push(' ');
            }
        }
        if !cur.trim().is_empty() && decl.is_empty() {
            decl = format!("<{}>", cur.trim());
        }
        if !cur.trim().is_empty() {
            params.push(cur.trim().to_string());
        }
    }
    (decl.clone(), decl, params)
}

#[proc_macro_derive(Component)]
pub fn derive_component(input: TokenStream) -> TokenStream {
    let name = type_name(input.clone());
    let (gdecl, _guse, _params) = generics_of(input);
    let out = format!("impl{gdecl} rf_ecs::Component for {name}{gdecl} {{}}");
    out.parse().expect("valid impl tokens")
}

#[proc_macro_derive(Resource)]
pub fn derive_resource(input: TokenStream) -> TokenStream {
    let name = type_name(input.clone());
    let (gdecl, _guse, _params) = generics_of(input);
    let out = format!("impl{gdecl} rf_ecs::Resource for {name}{gdecl} {{}}");
    out.parse().expect("valid impl tokens")
}
