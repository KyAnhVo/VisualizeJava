use crate::{
    name_resolution::resolve_types::{FlattenProject, FlattenType, Scope},
    types::{
        AccessModifier, ImportObject, JavaFile, Member, MemberKind, QualifiedName, RefType,
        TypeArg, TypeArgList, TypeParamList, VoidableType,
    },
};

pub fn resolve_ast(ast: &mut JavaFile, flatten_project: &FlattenProject) {}

fn setup_scope(ast: &JavaFile, project: &FlattenProject) -> Scope {
    let mut scope = Scope::new();
    setup_scope_wildcard_import(ast, project, &mut scope);

    scope
}

/// First: update wildcard imports
fn setup_scope_wildcard_import(ast: &JavaFile, project: &FlattenProject, scope: &mut Scope) {
    ast.imported_objects.iter().for_each(
        |ImportObject {
             name,
             is_static,
             is_wildcard,
         }| {
            match (is_static, is_wildcard) {
                (true, true) => {
                    // static wildcard, then the algorithm is:
                    // check if the whole name is valid fqn (and by that extend, extract the package)
                    // For each type in the same package:
                    //      if the name is not a prefix: continue
                    //      if there is no static keyword: continue
                    //      take the postfix: typename = name + postfix
                    //      scope[postfix] = typename
                    let Some((package, _typename)) =
                        seperate_type_name_and_package_nodefault(name, project)
                    else {
                        panic!("name not in project");
                    };
                    project
                        .get_package(&package)
                        .unwrap()
                        .iter()
                        .for_each(|file| {
                            file.iter().for_each(|typename| {
                                let modifiers = &file.get_type(typename).unwrap().modifiers;
                                if typename.has_prefix(name)
                                    && modifiers.modifiers.contains(&"static".to_owned())
                                    && modifiers.access_modifier == AccessModifier::Public
                                {
                                    let postfix = typename.get_suffix(name.0.len()).unwrap();
                                    scope.push(&postfix, typename);
                                }
                            })
                        });
                }
                (false, true) => {
                    // non-static, wildcard. name must be package.
                    assert!(
                        project.contains_package(name),
                        "project does not contain package"
                    );
                    project.get_package(name).unwrap().iter().for_each(|file| {
                        file.iter().for_each(|typeclass| {
                            let FlattenType {
                                name, modifiers, ..
                            } = file.get_type(typeclass).unwrap();
                            if modifiers.access_modifier != AccessModifier::Public {
                                return;
                            }
                            let (_, type_name) =
                                seperate_type_name_and_package_nodefault(name, project).unwrap();
                            scope.push(&type_name, name);
                        });
                    });
                }
                _ => {}
            }
        },
    );
}
fn resolve_member(member: &mut Member, scope: &mut Scope, project: &FlattenProject) {
    match &mut member.member_kind {
        MemberKind::Property { reftype, .. } => {
            resolve_reftype(reftype, scope, project);
        }
        MemberKind::Method {
            type_param_list,
            input,
            output,
            throws,
        } => {
            let updated_names = update_scope_generic(scope, type_param_list);
            input.iter_mut().for_each(|reftype| {
                resolve_reftype(reftype, scope, project);
            });
            match output {
                VoidableType::Void => {}
                VoidableType::RefType(reftype) => resolve_reftype(reftype, scope, project),
            }
            throws.iter_mut().for_each(|ref_type| {
                resolve_reftype(ref_type, scope, project);
            });
            updated_names.iter().for_each(|name| {
                assert!(scope.pop_uncheck(name) == *name);
            });
        }
        MemberKind::Constructor {
            type_param_list,
            input,
            throws,
        } => {
            let updated_names = update_scope_generic(scope, type_param_list);
            input.iter_mut().for_each(|reftype| {
                resolve_reftype(reftype, scope, project);
            });
            throws.iter_mut().for_each(|ref_type| {
                resolve_reftype(ref_type, scope, project);
            });
            updated_names.iter().for_each(|name| {
                assert!(scope.pop_uncheck(name) == *name);
            });
        }
    }
}

fn update_scope_generic(scope: &mut Scope, type_param_list: &TypeParamList) -> Vec<QualifiedName> {
    let mut v: Vec<QualifiedName> = vec![];
    for type_param in type_param_list.0.iter() {
        let name = QualifiedName(vec![type_param.name.clone()]);
        scope.push(&name, &name);
        v.push(name);
    }
    v
}

fn resolve_reftype(reftype: &mut RefType, scope: &Scope, project: &FlattenProject) {
    if !check_is_fqn(&reftype.name, project) {
        if let Some(s) = scope.get(&reftype.name) {
            reftype.name = s;
        }
    }
    resolve_type_arg_list(&mut reftype.type_arg_list, scope, project);
}

fn resolve_type_arg_list(type_arg_list: &mut TypeArgList, scope: &Scope, project: &FlattenProject) {
    for type_arg in type_arg_list.0.iter_mut() {
        match type_arg {
            TypeArg::Is(reftype) => {
                resolve_reftype(reftype, scope, project);
            }
            TypeArg::Extends(reftype) => {
                resolve_reftype(reftype, scope, project);
            }
            TypeArg::Super(reftype) => {
                resolve_reftype(reftype, scope, project);
            }
            TypeArg::Wildcard => {}
        };
    }
}

fn seperate_type_name_and_package_nodefault(
    name: &QualifiedName,
    project: &FlattenProject,
) -> Option<(QualifiedName, QualifiedName)> {
    let mut i = 1;
    while let Some(ref prefix) = name.get_prefix(i) {
        if project.contains_package(&prefix) && name_is_in_package(name, prefix, project).unwrap() {
            return Some((prefix.clone(), name.get_suffix(i).unwrap()));
        }
        i += 1;
    }
    None
}

fn check_is_fqn(name: &QualifiedName, project: &FlattenProject) -> bool {
    let mut i = 1;
    while let Some(ref prefix) = name.get_prefix(i) {
        i += 1;
        if project.contains_package(&prefix) && name_is_in_package(name, prefix, project).unwrap() {
            return true;
        }
    }
    false
}

fn name_is_in_package(
    name: &QualifiedName,
    package: &QualifiedName,
    project: &FlattenProject,
) -> Option<bool> {
    let package = project.get_package(package)?;
    for file in package.iter() {
        if file.contains(&name) {
            return Some(true);
        }
    }

    Some(false)
}
