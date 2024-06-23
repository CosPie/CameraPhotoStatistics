use rayon::prelude::*;
use std::borrow::Cow;
use std::collections::HashMap;
use std::error::Error;
use std::fs::File;
use std::fs::{self};
use std::io::BufReader;
use std::path::{Path, PathBuf};
use std::sync::Mutex;
use walkdir::WalkDir;

use exif::Tag;
use lazy_static::lazy_static;

const OUTPUT_TAG_LIST: &[Tag] = &[Tag::FocalLength, Tag::FNumber, Tag::ExposureTime];

pub type CountRecordType = Mutex<HashMap<Cow<'static, str>, i32>>;
pub type OutputRecordType = HashMap<&'static Tag, CountRecordType>;
lazy_static! {
    static ref OutputRecord: OutputRecordType = {
        let mut map = HashMap::new();
        for tag in OUTPUT_TAG_LIST {
            map.insert(tag, Mutex::new(HashMap::new()));
        }
        map
    };
    static ref YearOutputRecord: HashMap<i16, OutputRecordType> = {
        let mut map = HashMap::new();
        for year in 2000..=2022 {
            let mut inner_map = HashMap::new();
            for tag in OUTPUT_TAG_LIST {
                inner_map.insert(tag, Mutex::new(HashMap::new()));
            }
            map.insert(year, inner_map);
        }
        map
    };
    static ref JPGImageCount: Mutex<Box<i32>> = Mutex::new(Box::new(0));
    static ref AllFileCount: Mutex<Box<i32>> = Mutex::new(Box::new(0));
}

pub fn create_year_output_record<'a>(years: &[&'a str]) -> HashMap<&'a str, OutputRecordType> {
    let mut map = HashMap::new();
    for year in years {
        let mut inner_map = HashMap::new();
        for tag in OUTPUT_TAG_LIST {
            inner_map.insert(tag, Mutex::new(HashMap::new()));
        }
        map.insert(*year, inner_map);
    }
    map
}

pub fn create_output_record() -> OutputRecordType {
    let mut map = HashMap::new();
    for tag in OUTPUT_TAG_LIST {
        map.insert(tag, Mutex::new(HashMap::new()));
    }
    map
}

pub fn scan_exif_from_img(path: &Path, record: &OutputRecordType) -> Result<(), Box<dyn Error>> {
    let file = File::open(path)?;

    let mut bufreader = BufReader::new(&file);
    let exifreader = exif::Reader::new();
    let exif = exifreader.read_from_container(&mut bufreader)?;

    for f in exif.fields() {
        let exif_key = f.tag;
        if !OUTPUT_TAG_LIST.contains(&exif_key) {
            continue;
        }
        let exif_value = f.display_value().to_string();
        // ignore illegal
        if &exif_value == "0" {
            continue;
        }
        for (tag, record) in record.iter() {
            if *tag == &exif_key {
                let mut record = record.lock().unwrap();
                record
                    .entry(Cow::Owned(exif_value))
                    .and_modify(|count| *count += 1)
                    .or_insert(1);
                break;
            }
        }
    }
    **JPGImageCount.lock().unwrap() += 1;

    Ok(())
}

pub fn scan_exif_from_folder(folder_path: &Path, record: &OutputRecordType) {
    WalkDir::new(folder_path)
        .min_depth(0)
        .max_depth(4)
        .into_iter()
        .filter_map(|e| e.ok())
        .filter(|entry| entry.file_type().is_file())
        .inspect(|_| {
            **(AllFileCount.lock().unwrap()) += 1;
        })
        .filter(|entry| {
            entry
                .path()
                .extension()
                .unwrap_or_default()
                .to_ascii_lowercase()
                == "jpg"
        })
        .map(|entry| entry.into_path())
        .collect::<Vec<PathBuf>>()
        .par_iter()
        .for_each(|path| {
            let path = path.as_path();
            let _ = scan_exif_from_img(path, record);
        });
}

pub fn save_analyze_result(year: &str, year_record: &OutputRecordType) {
    let mut v = Vec::from(["{".to_owned()]);
    for (tag, record) in year_record.iter() {
        let result = format!(
            "{:?}:{:?},",
            tag.description().unwrap(),
            record.lock().unwrap()
        );
        println!("{}", result);
        v.push(result);
    }
    v.last_mut().unwrap().pop();
    v.push("}".to_owned());
    fs::write(
        Path::new(&format!(
            "F:\\Code\\aws-fn\\exif_reader\\report\\camera_photo_analyze_report{}.json",
            year
        )),
        v.join(""),
    )
    .expect("error write");
}
pub fn get_analyze_result(year_record: &OutputRecordType) -> Result<String, Box<dyn Error>> {
    let mut v = Vec::from(["{".to_owned()]);
    for (tag, record) in year_record.iter() {
        let result = format!(
            "{:?}:{:?},",
            tag.description().unwrap(),
            record.lock().unwrap()
        );
        println!("{}", result);
        v.push(result);
    }
    v.last_mut().unwrap().pop();
    v.push("}".to_owned());
    Ok(v.join(""))
}
#[test]
pub fn test_single_img() {
    let path = Path::new(r"./test/test.jpg");
    let output_record = create_output_record();
    scan_exif_from_img(path, &output_record).unwrap();
    let f_number = output_record.get(&Tag::FNumber).unwrap().lock().unwrap();
    assert_eq!(f_number.len(), 1)
}

#[test]
pub fn test_folder_img() {
    let path = Path::new(r"./test");
    let output_record = create_output_record();
    scan_exif_from_folder(path, &output_record);
    let f_number = output_record.get(&Tag::FNumber).unwrap().lock().unwrap();
    assert_eq!(f_number.len(), 2)
}
#[test]
pub fn test_generate_report_by_year() {
    let year_list = ["2022", "2023", "2024"];
    let year_output_record = create_year_output_record(&year_list);
    year_list.par_iter().for_each(|year| {
        let path = format!("E:\\CameraPhoto\\{}", year);
        let scan_folder_path = Path::new(&path);
        let current_year_record = year_output_record.get(*year).unwrap();
        scan_exif_from_folder(scan_folder_path, current_year_record);
    });

    for year_r in year_output_record.iter() {
        println!("{:?}year", year_r.0);
        year_r.1.iter().for_each(|data| {
            println!("{:?}", data);
        })
    }
}
