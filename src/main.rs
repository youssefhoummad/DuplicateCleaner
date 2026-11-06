#![windows_subsystem = "windows"]

extern crate native_windows_gui as nwg;
extern crate native_windows_derive as nwd;

use nwd::NwgUi;
use nwg::NativeUi;
use std::collections::{HashMap, HashSet};
use std::fs;
use std::path::{Path, PathBuf};
use walkdir::WalkDir;
use xxhash_rust::xxh3::xxh3_128;

#[derive(Default, NwgUi)]
pub struct DuplicateCleaner {
    #[nwg_control(size: (900, 600), position: (300, 300), title: "البحث عن الملفات المكررة (Rust - Native Win32)", flags: "WINDOW|VISIBLE")]
    #[nwg_events( OnWindowClose: [nwg::stop_thread_dispatch()] )]
    window: nwg::Window,

    #[nwg_layout(parent: window, margin: [10, 10, 10, 10], spacing: 5)]
    layout: nwg::GridLayout,

    // المجلد 1
    #[nwg_control(text: "المجلد 1 (الأساسي):")]
    #[nwg_layout_item(layout: layout, col: 2, row: 0)]
    lbl_folder1: nwg::Label,

    #[nwg_control(text: "")]
    #[nwg_layout_item(layout: layout, col: 1, row: 0)]
    txt_folder1: nwg::TextInput,

    #[nwg_control(text: "استعراض...")]
    #[nwg_layout_item(layout: layout, col: 0, row: 0)]
    #[nwg_events( OnButtonClick: [DuplicateCleaner::browse_folder1] )]
    btn_browse1: nwg::Button,

    // المجلد 2
    #[nwg_control(text: "المجلد 2 (اختياري):")]
    #[nwg_layout_item(layout: layout, col: 2, row: 1)]
    lbl_folder2: nwg::Label,

    #[nwg_control(text: "")]
    #[nwg_layout_item(layout: layout, col: 1, row: 1)]
    txt_folder2: nwg::TextInput,

    #[nwg_control(text: "استعراض...")]
    #[nwg_layout_item(layout: layout, col: 0, row: 1)]
    #[nwg_events( OnButtonClick: [DuplicateCleaner::browse_folder2] )]
    btn_browse2: nwg::Button,

    // الأزرار
    #[nwg_control(text: "فحص الملفات")]
    #[nwg_layout_item(layout: layout, col: 2, row: 2)]
    #[nwg_events( OnButtonClick: [DuplicateCleaner::scan_files] )]
    btn_scan: nwg::Button,

    #[nwg_control(text: "معاينة الملف المختار")]
    #[nwg_layout_item(layout: layout, col: 1, row: 2)]
    #[nwg_events( OnButtonClick: [DuplicateCleaner::preview_file] )]
    btn_preview: nwg::Button,

    #[nwg_control(text: "نقل المكرر للحجر")]
    #[nwg_layout_item(layout: layout, col: 0, row: 2)]
    #[nwg_events( OnButtonClick: [DuplicateCleaner::quarantine_files] )]
    btn_quarantine: nwg::Button,

    // قائمة النتائج
    #[nwg_control(list_style: nwg::ListViewStyle::Detailed, ex_flags: nwg::ListViewExFlags::GRID | nwg::ListViewExFlags::FULL_ROW_SELECT)]
    #[nwg_layout_item(layout: layout, col: 0, col_span: 3, row: 3, row_span: 5)]
    list_view: nwg::ListView,

    // الحالة
    #[nwg_control(text: "مستعد. اختر المجلدات واضغط فحص.")]
    #[nwg_layout_item(layout: layout, col: 0, col_span: 3, row: 8)]
    lbl_status: nwg::Label,
}

impl DuplicateCleaner {
    fn browse_folder1(&self) {
        if let Some(dir) = self.show_dir_dialog() {
            self.txt_folder1.set_text(&dir);
        }
    }

    fn browse_folder2(&self) {
        if let Some(dir) = self.show_dir_dialog() {
            self.txt_folder2.set_text(&dir);
        }
    }

    fn show_dir_dialog(&self) -> Option<String> {
        let mut dialog = nwg::FileDialog::default();
        nwg::FileDialog::builder()
            .action(nwg::FileDialogAction::OpenDirectory)
            .title("اختر مجلداً")
            .build(&mut dialog)
            .ok()?;
        
        if dialog.run(Some(&self.window)) {
            if let Ok(dir) = dialog.get_selected_item() {
                return Some(dir.into_string().unwrap());
            }
        }
        None
    }

    fn scan_files(&self) {
        let f1 = self.txt_folder1.text();
        let f2 = self.txt_folder2.text();

        if f1.is_empty() {
            nwg::modal_error_message(&self.window, "تنبيه", "يرجى اختيار المجلد 1 أولاً.");
            return;
        }

        self.list_view.clear();
        self.lbl_status.set_text("جارٍ الفحص وحساب البصمات (xxHash)... يرجى الانتظار.");
        
        // إعداد أعمدة القائمة إذا كانت فارغة - تم تصحيح الثوابت إلى حروف كبيرة
        if self.list_view.column_len() == 0 {
            self.list_view.insert_column(nwg::InsertListViewColumn { index: Some(0), fmt: Some(nwg::ListViewColumnFlags::LEFT), width: Some(500), text: Some("مسار الملف".to_string()) });
            self.list_view.insert_column(nwg::InsertListViewColumn { index: Some(1), fmt: Some(nwg::ListViewColumnFlags::LEFT), width: Some(100), text: Some("الإجراء".to_string()) });
            self.list_view.insert_column(nwg::InsertListViewColumn { index: Some(2), fmt: Some(nwg::ListViewColumnFlags::LEFT), width: Some(100), text: Some("الحجم".to_string()) });
        }

        let is_single = f2.is_empty();
        let mut duplicates = Vec::new();

        if is_single {
            duplicates = self.find_single(&f1);
        } else {
            duplicates = self.find_two(&f1, &f2);
        }

        self.populate_list(duplicates);
        self.lbl_status.set_text("اكتمل الفحص.");
    }

    fn hash_file(path: &Path) -> Option<u128> {
        if let Ok(bytes) = fs::read(path) {
            Some(xxh3_128(&bytes))
        } else {
            None
        }
    }

    fn find_single(&self, path: &str) -> Vec<Vec<PathBuf>> {
        let mut by_size: HashMap<u64, Vec<PathBuf>> = HashMap::new();
        for entry in WalkDir::new(path).into_iter().filter_map(|e| e.ok()) {
            if entry.file_type().is_file() {
                if let Ok(metadata) = entry.metadata() {
                    let size = metadata.len();
                    if size > 0 {
                        by_size.entry(size).or_default().push(entry.into_path());
                    }
                }
            }
        }

        let mut by_hash: HashMap<u128, Vec<PathBuf>> = HashMap::new();
        for (_, files) in by_size.into_iter().filter(|(_, v)| v.len() > 1) {
            for file in files {
                if let Some(hash) = Self::hash_file(&file) {
                    by_hash.entry(hash).or_default().push(file);
                }
            }
        }

        by_hash.into_values().filter(|v| v.len() > 1).collect()
    }

    fn find_two(&self, p1: &str, p2: &str) -> Vec<Vec<PathBuf>> {
        let mut ref_hashes = HashSet::new();
        for entry in WalkDir::new(p1).into_iter().filter_map(|e| e.ok()) {
            if entry.file_type().is_file() {
                if let Some(hash) = Self::hash_file(entry.path()) {
                    ref_hashes.insert(hash);
                }
            }
        }

        let mut found: HashMap<u128, Vec<PathBuf>> = HashMap::new();
        for entry in WalkDir::new(p2).into_iter().filter_map(|e| e.ok()) {
            if entry.file_type().is_file() {
                if let Some(hash) = Self::hash_file(entry.path()) {
                    if ref_hashes.contains(&hash) {
                        found.entry(hash).or_default().push(entry.into_path());
                    }
                }
            }
        }
        found.into_values().collect()
    }

    fn populate_list(&self, groups: Vec<Vec<PathBuf>>) {
        let mut row_index: i32 = 0; // تعديل نوع البيانات ليتوافق مع الإدخال
        for group in groups {
            // الأصل
            self.list_view.insert_item(nwg::InsertListViewItem { index: Some(row_index), column_index: 0, text: Some(group[0].to_string_lossy().to_string()), image: None });
            // تم تصحيح تحويل النوع هنا
            self.list_view.update_item(row_index as usize, nwg::InsertListViewItem { index: Some(row_index), column_index: 1, text: Some("يُحتفظ به".to_string()), image: None });
            row_index += 1;

            // المكررات (النقل)
            for file in group.iter().skip(1) {
                self.list_view.insert_item(nwg::InsertListViewItem { index: Some(row_index), column_index: 0, text: Some(file.to_string_lossy().to_string()), image: None });
                // تم تصحيح تحويل النوع هنا
                self.list_view.update_item(row_index as usize, nwg::InsertListViewItem { index: Some(row_index), column_index: 1, text: Some("سيُنقل".to_string()), image: None });
                row_index += 1;
            }
        }
    }

    fn preview_file(&self) {
        if let Some(index) = self.list_view.selected_item() {
            if let Some(path_str) = self.list_view.item(index, 0, 512) {
                let _ = opener::open(path_str.text);
            }
        } else {
            nwg::modal_info_message(&self.window, "تنبيه", "اختر ملفاً من القائمة أولاً لمعاينته.");
        }
    }

    fn quarantine_files(&self) {
        nwg::modal_info_message(&self.window, "معلومة", "تم التعرف على الملفات بنجاح. دالة النقل يمكن إضافتها لتقوم بتحريك الملفات إلى مجلد Quarantine.");
    }
}

fn main() {
    nwg::init().expect("فشل في تهيئة واجهة Win32");
    nwg::Font::set_global_family("Segoe UI").expect("فشل تعيين الخط");
    let _app = DuplicateCleaner::build_ui(Default::default()).expect("فشل بناء الواجهة");
    nwg::dispatch_thread_events();
}
