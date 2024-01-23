use std::fmt::Write;

use clang::{Clang, Entity, EntityKind, EvaluationResult, Index, Type, TypeKind};

#[derive(Default)]
struct Wranglings {
    commands_xml: String,
    types_xml: String,
    enums_xml: String,
    commands: Vec<String>,
    types: Vec<String>,
    extension_id: Option<i64>,
    extension_name: Option<String>,
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<String> = std::env::args().collect();
    let input_file = std::path::PathBuf::from(&args[1]).canonicalize()?;

    let mut w = Wranglings::default();

    let clang = Clang::new()?;
    let index = Index::new(&clang, false, false);
    let tu = index
        .parser(&args[1])
        .detailed_preprocessing_record(true)
        .parse()?;
    let toplevels = tu.get_entity().get_children();
    for tl in toplevels {
        let Some(location) = tl.get_location() else {
            continue;
        };
        let Some(file) = location.get_file_location().file else {
            continue;
        };
        if file.get_path().canonicalize().unwrap() != input_file {
            continue;
        }
        match tl.get_kind() {
            EntityKind::TypedefDecl => {
                handle_typedef(&mut w, tl);
            }
            EntityKind::VarDecl => {
                handle_vardecl(&mut w, tl);
            }
            EntityKind::MacroDefinition => {
                handle_macro(&mut w, tl);
            }
            _ => {
                // dbg!(tl);
            }
        }
    }

    println!("--- Paste this before the </commands> closer ---");
    println!("<!-- XXX: lol -->");
    print!("{}", w.commands_xml);
    println!();
    println!("--- Paste this before the </types> closer ---");
    println!("<!-- XXX: lol -->");
    print!("{}", w.types_xml);
    println!();

    println!("--- Paste this before the </extensions> closer ---");
    println!("<!-- XXX: lol -->");
    println!(
        r#"<extension name="{}" number="{}" type="instance" supported="openxr">"#,
        w.extension_name.unwrap(),
        w.extension_id.unwrap()
    );
    println!(r#"<require>"#);
    println!(r#"{}"#, w.enums_xml);
    for cmd in w.commands {
        println!(r#"<command name="{cmd}"/>"#);
    }
    for ty in w.types {
        println!(r#"<type name="{ty}"/>"#);
    }
    println!(r#"</require>"#);
    println!(r#"</extension>"#);
    Ok(())
}

fn handle_typedef(w: &mut Wranglings, e: Entity<'_>) {
    let ty = e.get_typedef_underlying_type().unwrap();
    match ty.get_kind() {
        TypeKind::Pointer
            if ty.get_pointee_type().as_ref().map(Type::get_kind)
                == Some(TypeKind::FunctionPrototype) =>
        {
            let func = ty.get_pointee_type().unwrap();
            let name = &e.get_display_name().unwrap()[4..];
            let ret_ty = func.get_result_type().unwrap();
            let arg_tys = func.get_argument_types().unwrap();
            let arg_names = e.get_children()[1..]
                .iter()
                .map(Entity::get_display_name)
                .map(Option::unwrap)
                .collect::<Vec<_>>();

            w.commands.push(name.to_owned());

            writeln!(
                w.commands_xml,
                r#"<command successcodes="XR_SUCCESS" errorcodes="XR_ERROR_FUNCTION_UNSUPPORTED">"#
            );
            writeln!(
                w.commands_xml,
                r#"<proto><type>{}</type><name>{name}</name></proto>"#,
                ret_ty.get_display_name()
            );
            for (ty, name) in arg_tys.into_iter().zip(&arg_names) {
                writeln!(
                    w.commands_xml,
                    r#"<param>{}<name>{name}</name></param>"#,
                    format_type(&ty)
                );
            }
            writeln!(w.commands_xml, r#"</command>"#);
        }
        TypeKind::Elaborated => {
            let name = ty.get_display_name();
            let name = name.trim_start_matches("struct ");
            let fields = ty.get_elaborated_type().unwrap().get_fields().unwrap();

            w.types.push(name.to_owned());

            writeln!(w.types_xml, r#"<type category="struct" name="{name}">"#);
            for field in fields {
                let name = field.get_display_name().unwrap();
                let ty = field.get_type().unwrap();
                writeln!(
                    w.types_xml,
                    r#"<member>{} <name>{name}</name></member>"#,
                    format_type(&ty)
                );
            }
            writeln!(w.types_xml, r#"</type>"#);
        }
        _ => {}
    }
    println!();
}

fn format_type(ty: &Type<'_>) -> String {
    if ty.get_kind() == TypeKind::Pointer {
        let pointee = ty.get_pointee_type().unwrap();
        let qual = if pointee.is_const_qualified() {
            "const "
        } else {
            ""
        };
        let inner = format_type(&pointee);
        format!("{qual}{inner}*")
    } else {
        format!(
            "<type>{}</type>",
            ty.get_display_name().trim_start_matches("const ")
        )
    }
}

fn handle_vardecl(w: &mut Wranglings, tl: Entity<'_>) {
    let children = tl.get_children();
    let [ty, .., initialiser] = &*children else {
        panic!("nuh uh")
    };
    let [.., initialiser] = &*initialiser.get_children() else {
        panic!("no")
    };
    let EvaluationResult::SignedInteger(mut x) = initialiser.evaluate().unwrap() else {
        return;
    };
    x -= 1_000_000_000;
    let ext_id = x / 1000;
    match w.extension_id {
        Some(v) if v == ext_id => {}
        Some(_) => panic!("uh oh"),
        None => w.extension_id = Some(ext_id),
    }

    let offset = x % 1000;
    writeln!(
        w.enums_xml,
        r#"<enum offset="{offset}" extends="{}" name="{}"/>"#,
        ty.get_display_name().unwrap(),
        tl.get_name().unwrap(),
    );
}

fn handle_macro(w: &mut Wranglings, tl: Entity<'_>) {
    let name = tl.get_display_name().unwrap();
    let val = tl
        .get_range()
        .unwrap()
        .tokenize()
        .last()
        .unwrap()
        .get_spelling();
    if name.ends_with("_SPEC_VERSION") {
        writeln!(w.enums_xml, r#"<enum value="{val}" name="{name}"/>"#);
    } else if name.ends_with("_EXTENSION_NAME") {
        w.extension_name = Some(val.trim_matches('"').to_owned());
    }
}
