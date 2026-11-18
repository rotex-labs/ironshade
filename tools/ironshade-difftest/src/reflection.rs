use ironshade_artifact::{
    Binding, BindingPoint, BindingUse, EntryPoint, InterfaceVariable, LayoutSize, MemberLayout,
    Reflection, TypeLayout,
};
use ironshade_base::ShaderStage;
use serde_json::Value;

use crate::Error;

pub(crate) fn normalize(
    contents: &[u8],
    name: &str,
    stage: ShaderStage,
) -> Result<Reflection, Error> {
    let raw: Value = serde_json::from_slice(contents)?;
    if raw["version"].as_str() != Some("1.1") {
        return Err("expected slangc reflection document version 1.1".into());
    }
    let entries = raw["entryPoints"]
        .as_array()
        .ok_or("reflection has no entry-point array")?;
    let entry = entries
        .iter()
        .find(|entry| {
            entry["name"].as_str() == Some(name) && entry["stage"].as_str() == Some(stage.as_str())
        })
        .ok_or_else(|| {
            format!(
                "missing {} entry point {name} in slangc reflection",
                stage.as_str()
            )
        })?;
    let parameters = raw["parameters"]
        .as_array()
        .ok_or("reflection has no parameter array")?;
    let mut declared_bindings = Vec::new();
    let mut blocks = Vec::new();
    for value in parameters {
        let binding = normalize_binding(value)?;
        if class(&value["binding"]) == Some("pushConstantBuffer")
            || binding.ty.kind == "constantBuffer"
        {
            blocks.push(binding.clone());
        }
        declared_bindings.push(binding);
    }

    let mut inputs = Vec::new();
    let mut outputs = Vec::new();
    for parameter in items(&entry["parameters"]) {
        let direction = class(&parameter["binding"]);
        if direction != Some("varyingOutput") {
            interface(parameter, "parameter", "varyingInput", &mut inputs)?;
        }
        // Structs and inout parameters can carry output bindings on their members.
        if direction == Some("varyingOutput") || has_output(parameter) {
            interface(parameter, "parameter", "varyingOutput", &mut outputs)?;
        }
    }
    if let Some(result) = entry.get("result") {
        interface(result, "result", "varyingOutput", &mut outputs)?;
    }
    let workgroup_size = entry
        .get("threadGroupSize")
        .map(|value| serde_json::from_value(value.clone()))
        .transpose()?;
    let live_bindings = entry["bindings"]
        .as_array()
        .map(|bindings| {
            bindings
                .iter()
                .map(|binding| {
                    Ok(BindingUse {
                        name: required(&binding["name"], "entry binding name")?.to_owned(),
                        used: binding["binding"]["used"].as_bool().or_else(|| {
                            binding["binding"]["used"].as_u64().map(|value| value != 0)
                        }),
                    })
                })
                .collect::<Result<Vec<_>, Error>>()
        })
        .transpose()?;

    // The oracle reports source or intermediate bindings. For example its WGSL
    // output can use immediates while reflection still reports push constants.
    // Keep physical coordinates unknown unless a producer supplies them.
    Ok(Reflection {
        entry_points: vec![EntryPoint {
            name: name.to_owned(),
            stage,
            workgroup_size,
            inputs,
            outputs,
            live_bindings,
        }],
        declared_bindings,
        blocks,
        ..Reflection::default()
    })
}

fn normalize_binding(value: &Value) -> Result<Binding, Error> {
    let ty = layout(&value["type"])?;
    Ok(Binding {
        name: required(&value["name"], "resource name")?.to_owned(),
        kind: string(&value["type"]["baseShape"]).unwrap_or_else(|| ty.kind.clone()),
        access: string(&value["type"]["access"]),
        array_size: (ty.kind == "array").then_some(ty.element_count).flatten(),
        logical: binding_point(&value["binding"]),
        physical: None,
        ty,
    })
}

fn binding_point(value: &Value) -> Option<BindingPoint> {
    Some(BindingPoint {
        class: class(value)?.to_owned(),
        space: uint(&value["space"]),
        index: uint(&value["index"])?,
    })
}

fn class(value: &Value) -> Option<&str> {
    value["kind"].as_str()
}

fn has_output(value: &Value) -> bool {
    class(&value["binding"]) == Some("varyingOutput")
        || items(&value["type"]["fields"]).iter().any(has_output)
}

fn interface(
    value: &Value,
    fallback: &str,
    direction: &str,
    interface: &mut Vec<InterfaceVariable>,
) -> Result<(), Error> {
    let name = value["name"].as_str().unwrap_or(fallback);
    let children = items(&value["type"]["fields"]);
    if !children.is_empty() {
        for member in children {
            let child = required(&member["name"], "interface member name")?;
            let qualified = format!("{name}.{child}");
            interface_member(member, &qualified, direction, interface)?;
        }
        return Ok(());
    }
    interface_leaf(value, name, direction, interface)
}

fn interface_member(
    value: &Value,
    qualified: &str,
    direction: &str,
    interface: &mut Vec<InterfaceVariable>,
) -> Result<(), Error> {
    let children = items(&value["type"]["fields"]);
    if children.is_empty() {
        return interface_leaf(value, qualified, direction, interface);
    }
    for member in children {
        let name = required(&member["name"], "interface member name")?;
        interface_member(member, &format!("{qualified}.{name}"), direction, interface)?;
    }
    Ok(())
}

fn interface_leaf(
    value: &Value,
    name: &str,
    direction: &str,
    interface: &mut Vec<InterfaceVariable>,
) -> Result<(), Error> {
    let binding = &value["binding"];
    if class(binding).is_some_and(|kind| kind != direction) {
        return Ok(());
    }
    let semantic = value["semanticName"].as_str();
    let builtin = match semantic {
        Some("SV_POSITION") => Some("position"),
        Some("SV_POINTSIZE") => Some("point_size"),
        Some("SV_POINTCOORD") => Some("point_coord"),
        Some("SV_DISPATCHTHREADID") => Some("global_invocation_id"),
        _ => None,
    };
    interface.push(InterfaceVariable {
        name: name.to_owned(),
        location: (class(binding) == Some(direction))
            .then(|| uint(&binding["index"]))
            .flatten(),
        semantic: semantic.map(str::to_owned),
        semantic_index: uint(&value["semanticIndex"]),
        builtin: builtin.map(str::to_owned),
        interpolation: string(&value["interpolation"]),
        ty: layout(&value["type"])?,
    });
    Ok(())
}

fn layout(raw: &Value) -> Result<TypeLayout, Error> {
    let sizes = items(&raw["sizes"])
        .iter()
        .map(|value| {
            Ok(LayoutSize {
                class: required(&value["kind"], "layout size class")?.to_owned(),
                size: value["value"].as_u64(),
                alignment: value["alignment"].as_u64(),
            })
        })
        .collect::<Result<_, Error>>()?;
    let fields = items(&raw["fields"])
        .iter()
        .map(|field| {
            // Slang can report separate layouts for input, output, and uniform
            // storage. Offsets and strides here are buffer layout measurements.
            let binding = &field["binding"];
            let uniform = (class(binding) == Some("uniform"))
                .then_some(binding)
                .or_else(|| {
                    items(&field["bindings"])
                        .iter()
                        .find(|entry| class(entry) == Some("uniform"))
                });
            Ok(MemberLayout {
                name: required(&field["name"], "type member name")?.to_owned(),
                offset: uniform.and_then(|value| value["offset"].as_u64()),
                size: uniform.and_then(|value| value["size"].as_u64()),
                stride: uniform.and_then(|value| value["elementStride"].as_u64()),
                ty: layout(&field["type"])?,
            })
        })
        .collect::<Result<_, Error>>()?;
    let element = raw
        .get("elementType")
        .or_else(|| raw.get("resultType"))
        .map(|value| layout(value).map(Box::new))
        .transpose()?;
    Ok(TypeLayout {
        kind: required(&raw["kind"], "type kind")?.to_owned(),
        name: string(&raw["name"]),
        scalar: string(&raw["scalarType"]),
        element_count: raw["elementCount"].as_u64(),
        rows: uint(&raw["rowCount"]),
        columns: uint(&raw["columnCount"]),
        matrix_layout: string(&raw["matrixLayout"]),
        sizes,
        element,
        fields,
    })
}

fn items(value: &Value) -> &[Value] {
    value.as_array().map(Vec::as_slice).unwrap_or_default()
}

fn uint(value: &Value) -> Option<u32> {
    u32::try_from(value.as_u64()?).ok()
}

fn string(value: &Value) -> Option<String> {
    value.as_str().map(str::to_owned)
}

fn required<'a>(value: &'a Value, name: &str) -> Result<&'a str, Error> {
    value
        .as_str()
        .ok_or_else(|| format!("missing {name} in slangc reflection").into())
}

#[cfg(test)]
mod tests {
    use ironshade_artifact::{Artifact, Payload, TargetIdentity};

    use super::*;

    #[test]
    fn nova_layout_and_liveness_survive_artifact_json_roundtrip() {
        let snapshot: Value = serde_json::from_str(include_str!(
            "../../../tests/snapshots/nova/nova_gravity-computeMain-spirv.json"
        ))
        .unwrap();
        let reflection: Reflection =
            serde_json::from_value(snapshot["reflection"].clone()).unwrap();
        let entry = &reflection.entry_points[0];
        assert_eq!(entry.workgroup_size, Some([256, 1, 1]));
        assert_eq!(reflection.declared_bindings.len(), 4);
        let liveness = entry.live_bindings.as_ref().unwrap();
        assert_eq!(
            liveness
                .iter()
                .find(|binding| binding.name == "pc")
                .unwrap()
                .used,
            None
        );
        assert_eq!(
            liveness
                .iter()
                .find(|binding| binding.name == "outp")
                .unwrap()
                .used,
            Some(true)
        );

        let layout = reflection.blocks[0].ty.element.as_ref().unwrap();
        assert_eq!(layout.sizes[0].size, Some(64));
        assert_eq!(
            layout
                .fields
                .iter()
                .map(|field| field.offset.unwrap())
                .collect::<Vec<_>>(),
            (0..64).step_by(4).collect::<Vec<_>>(),
        );
        let artifact = Artifact {
            target: TargetIdentity {
                namespace: "example".into(),
                name: "test".into(),
                revision: 1,
                representation: "spirv".into(),
                environment: "vulkan1.1".into(),
            },
            payload: Payload::Binary(vec![3, 2, 35, 7]),
            reflection,
        };
        let json = serde_json::to_value(&artifact).unwrap();
        assert_eq!(
            serde_json::from_value::<Artifact>(json.clone()).unwrap(),
            artifact
        );
        let mut wrong_version = json;
        wrong_version["reflection"]["schema_version"] = 1.into();
        assert!(serde_json::from_value::<Artifact>(wrong_version).is_err());
    }
}
