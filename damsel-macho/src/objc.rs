use damsel_core::{ObjcMetadata, Section};

pub(crate) fn collect_objc_metadata(bytes: &[u8], sections: &[Section]) -> ObjcMetadata {
    let mut metadata = ObjcMetadata::default();

    for section in sections {
        let Some(offset) = section.file_offset else {
            continue;
        };
        let start = offset as usize;
        let end = start.saturating_add(section.file_size as usize);
        let Some(contents) = bytes.get(start..end) else {
            continue;
        };

        match section.name.as_str() {
            "__objc_classname" => metadata.class_names.extend(read_c_strings(contents)),
            "__objc_methname" => {
                let names = read_c_strings(contents);
                metadata.method_names.extend(names.clone());
                metadata.selector_names.extend(names);
            }
            "__objc_imageinfo" if contents.len() >= 8 => {
                metadata.image_info_flags = Some(u32::from_le_bytes([
                    contents[4],
                    contents[5],
                    contents[6],
                    contents[7],
                ]));
            }
            _ => {}
        }
    }

    metadata.class_names.sort();
    metadata.class_names.dedup();
    metadata.method_names.sort();
    metadata.method_names.dedup();
    metadata.selector_names.sort();
    metadata.selector_names.dedup();
    metadata
}

fn read_c_strings(bytes: &[u8]) -> Vec<String> {
    bytes
        .split(|byte| *byte == 0)
        .filter(|slice| !slice.is_empty())
        .map(|slice| String::from_utf8_lossy(slice).into_owned())
        .collect()
}
