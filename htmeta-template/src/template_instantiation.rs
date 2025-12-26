use crate::{CHILDREN, Template, TemplatePlugin, find, for_each_mut};

use std::{
    borrow::Cow,
    cell::{RefCell, RefMut},
    collections::{HashMap, HashSet},
    path::{Path, PathBuf},
    rc::Rc,
};

use htmeta::{
    EmitResult, HtmlEmitter,
    kdl::{KdlDocument, KdlEntry, KdlIdentifier, KdlNode, KdlValue},
    plugins::{EmitStatus, IPlugin, PluginContext},
};

use crate::utils::*;

/// Inserts `children_node`'s named entries into every direct `children`.
fn instantiate_children(children_node: KdlNode, children: &Vec<KdlNode>) -> Vec<KdlNode> {
    let entries = children_node.entries();
    let mut children = children.clone();
    for child in &mut children {
        // TODO: invert children, push entries at the end, then invert again for
        // better perf.
        for entry in entries {
            let Some(entry_name) = entry.name().map(KdlIdentifier::value) else {
                continue;
            };
            if child.entry_mut(entry_name).is_none() {
                child.entries_mut().insert(0, entry.clone());
            }
        }
    }
    children
}

fn set_variables<'a, 'b>(
    template: &'b Template,
    template_node: &mut KdlNode,
    context: &PluginContext,
    subemitter: &mut HtmlEmitter<'a>,
    instantiation_information: &'a KdlNode,
) -> EmitResult {
    let default_params = template
        .default_params()
        .map(|(key, val)| {
            Ok((
                key.to_owned(),
                context.emitter.vars.expand_value(val)?.as_owned(),
            ))
    }).collect::<EmitResult<Vec<_>>>()?;
    subemitter
        .vars
        .extend(default_params);

    // Turns named entries into variables with the corresponding name
    // (E.g: `arg="value"` => `$arg "value"`)
    //
    // And positional arguments into numbered variables
    // (E.g: `"foo" entry="zoo" "bar"` => `$0 "foo"; $entry "zoo"; $1 "bar"`)
    let args =
        instantiation_information
            .keyed_entries()
            .map(|(key, value)| {
                Ok((
                    Cow::<str>::from(key),
                    // value.clone()
                    context.emitter.vars.expand_value_str(value)?,
                ))
            }).collect::<EmitResult<Vec<_>>>()?;
    subemitter.vars.extend(
        args
    );

    // Creates special variable `props` which contains all unused properties.
    let props = instantiation_information
        .entries()
        .iter()
        .filter_map(|e| {
            let name = e.name()?;
            if template.is_param(name.value()) {
                None
            } else {
                Some(e.to_string().trim().to_owned())
            }
        })
        .collect::<Vec<_>>()
        .join(" ");

    subemitter.vars.insert("props", props.clone());

    let template = template_node.ensure_children();

    // If template only has one child, automatically apply $props
    if template.nodes().len() == 1 {
        let node = &mut template.nodes_mut()[0];
        let entries = node.entries_mut();
        if let Some(last) = entries.last()
            && last.name().is_none()
        {
            entries.insert(entries.len() - 1, props.into());
        } else {
            entries.push(props.into());
        }
    }
    Ok(())
}

impl TemplatePlugin {
    pub(crate) fn emit_template(
        &self,
        name: &str,
        instantiation_information: &KdlNode,
        context: PluginContext,
    ) -> EmitResult {
        let mut subemitter = context.emitter.clone();

        let template = &self.templates.borrow()[name];

        if instantiation_information.children().is_some() && !template.uses_children {
            return Err(format!(
                "{name}: Template was called with children but does not support it!"
            ))?;
        }
        if find(instantiation_information, &|node| {
            node.is_command("children")
        }) {
            return Err(format!(
                "{name}: Template call contain @children. Infinite recursion detected."
            ))?;
        }
        let template_children = instantiation_information
            .iter_children()
            .cloned()
            .collect::<Vec<_>>();

        // Recursively replaces @children with the children block.
        let mut template_node = template.node.clone();
        for_each_mut(&mut template_node, &|node| {
            let Some(children) = node.children_mut().as_mut() else {
                return;
            };
            let children = children.nodes_mut();
            *children = std::mem::take(children)
                .into_iter()
                .flat_map(|c| {
                    if c.is_command(CHILDREN) {
                        instantiate_children(c, &template_children)
                    } else {
                        vec![c]
                    }
                })
                .collect();
        });

        set_variables(
            template,
            &mut template_node,
            &context,
            &mut subemitter,
            instantiation_information,
        );

        subemitter.emit(template_node.children().unwrap(), context.writer)?;
        Ok(())
    }
}
