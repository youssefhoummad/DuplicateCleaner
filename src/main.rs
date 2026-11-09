#![windows_subsystem = "windows"]

extern crate native_windows_gui as nwg;
extern crate native_windows_derive as nwd;

use nwd::NwgUi;
use nwg::NativeUi;
use std::cell::RefCell;
use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};
use walkdir::WalkDir;
use xxhash_rust::xxh3::Xxh3;
use std::io::Read;
use chrono::Local;

#[derive(Clone, Debug)]
pub struct FileGroup {
    pub digest: String,
    pub size: u64,
    pub keep: PathBuf,
    pub move_files: Vec<PathBuf>,
    pub locked: bool,
}

#[derive(Default, NwgUi)]
pub struct DuplicateCleaner {
    #[nwg_control(size: (950, 700), position: (300, 200), title: "البحث عن الملفات المكررة (Rust Native)", flags: "WINDOW|VISIBLE")]
    #[nwg_events( OnWindowClose: [nwg::stop_thread_dispatch()] )]
    window: nwg::Window,

    #[nwg_layout(parent: window, margin: [10, 10, 10, 10], spacing: 5)]
    layout: nwg::GridLayout,

    #[nwg_control(text: "استعراض...")]
    #[nwg_layout_item(layout: layout, col: 0, row: 0)]
    #[nwg_events( OnButtonClick: [DuplicateCleaner::browse_folder1] )]
    btn_browse1: nwg::Button,

    #[nwg_control(text: "")]
    #[nwg_layout_item(layout: layout, col: 1, col_span: 4, row: 0)]
    txt_folder1: nwg::TextInput,

    #[nwg_control(text: "المجلد المرجع:")]
    #[nwg_layout_item(layout: layout, col: 5, row: 0)]
    lbl_folder1: nwg::Label,

    #[nwg_control(text: "استعراض...")]
    #[nwg_layout_item(layout: layout, col: 0, row: 1)]
    #[nwg_events( OnButtonClick: [DuplicateCleaner::browse_folder2] )]
    btn_browse2: nwg::Button,

    #[nwg_control(text: "")]
    #[nwg_layout_item(layout: layout, col: 1, col_span: 4, row: 1)]
    txt_folder2: nwg::TextInput,

    #[nwg_control(text: "مجلد المقارنة (اختياري):")]
    #[nwg_layout_item(layout: layout, col: 5, row: 1)]
    lbl_folder2: nwg::Label,

    #[nwg_control(text: "مسح النتائج")]
    #[nwg_layout_item(layout: layout, col: 0, row: 2)]
    #[nwg_events( OnButtonClick: [DuplicateCleaner::clear_results] )]
    btn_clear: nwg::Button,

    #[nwg_control(text: "احتفظ بهذا الملف")]
    #[nwg_layout_item(layout: layout, col: 1, row: 2)]
    #[nwg_events( OnButtonClick: [DuplicateCleaner::make_original] )]
    btn_make_orig: nwg::Button,

    #[nwg_control(text: "معاينة الملف")]
    #[nwg_layout_item(layout: layout, col: 2, row: 2)]
    #[nwg_events( OnButtonClick: [DuplicateCleaner::preview_file] )]
    btn_preview: nwg::Button,

    #[nwg_control(text: "ترتيب المجلدات ⭐")]
    #[nwg_layout_item(layout: layout, col: 3, row: 2)]
    #[nwg_events( OnButtonClick: [DuplicateCleaner::open_smart_window] )]
    btn_smart: nwg::Button,

    #[nwg_control(text: "نقل للحجر")]
    #[nwg_layout_item(layout: layout, col: 4, row: 2)]
    #[nwg_events( OnButtonClick: [DuplicateCleaner::quarantine_files] )]
    btn_quarantine: nwg::Button,

    #[nwg_control(text: "فحص الملفات")]
    #[nwg_layout_item(layout: layout, col: 5, row: 2)]
    #[nwg_events( OnButtonClick: [DuplicateCleaner::scan_files] )]
    btn_scan: nwg::Button,

    #[nwg_control(list_style: nwg::ListViewStyle::Detailed, ex_flags: nwg::ListViewExFlags::GRID | nwg::ListViewExFlags::FULL_ROW_SELECT)]
    #[nwg_layout_item(layout: layout, col: 0, col_span: 6, row: 3, row_span: 8)]
    list_view: nwg::ListView,

    #[nwg_control(text: "مستعد.")]
    #[nwg_layout_item(layout: layout, col: 0, col_span: 6, row: 11)]
    lbl_status: nwg::Label,

    // Smart Selection Dialog
    #[nwg_control(title: "ترتيب أولويات المجلدات", size: (500, 400), flags: "WINDOW")]
    smart_window: nwg::Window,

    #[nwg_layout(parent: smart_window, margin: [10, 10, 10, 10], spacing: 5)]
    smart_layout: nwg::GridLayout,

    #[nwg_control(parent: smart_window)]
    #[nwg_layout_item(layout: smart_layout, col: 0, col_span: 3, row: 0, row_span: 4)]
    smart_list: nwg::ListBox<String>,

    #[nwg_control(parent: smart_window, text: "▲ أعلى")]
    #[nwg_layout_item(layout: smart_layout, col: 3, row: 0)]
    #[nwg_events( OnButtonClick: [DuplicateCleaner::smart_up] )]
    btn_up: nwg::Button,

    #[nwg_control(parent: smart_window, text: "▼ أسفل")]
    #[nwg_layout_item(layout: smart_layout, col: 3, row: 1)]
    #[nwg_events( OnButtonClick: [DuplicateCleaner::smart_down] )]
    btn_down: nwg::Button,

    #[nwg_control(parent: smart_window, text: "تطبيق")]
    #[nwg_layout_item(layout: smart_layout, col: 3, row: 2)]
    #[nwg_events( OnButtonClick: [DuplicateCleaner::smart_apply] )]
    btn_apply: nwg::Button,

    // State Variables
    groups: RefCell<Vec<FileGroup>>,
    base_folder: RefCell<String>,
    is_single: RefCell<bool>,
    row_map: RefCell<HashMap<usize, (usize, bool)>>,
    smart_folder_map: RefCell<HashMap<String, String>>,
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
            .build(&mut dialog).ok()?;
        
        if dialog.run(Some(&self.window)) {
            if let Ok(dir) = dialog.get_selected_item() {
                return Some(dir.into_string().unwrap());
            }
        }
        None
    }

    fn hash_file(path: &Path) -> Option<String> {
        let mut file = std::fs::File::open(path).ok()?;
        let mut hasher = Xxh3::new();
        let mut buffer = [0; 65536];
        loop {
            let n = file.read(&mut buffer).ok()?;
            if n == 0 { break; }
            hasher.update(&buffer[..n]);
        }
        Some(format!("{:x}", hasher.digest()))
    }

    fn format_size(mut bytes: f64) -> String {
        let units = ["B", "KB", "MB", "GB", "TB"];
        let mut i = 0;
        while bytes >= 1024.0 && i < units.len() - 1 {
            bytes /= 1024.0;
            i += 1;
        }
        if i == 0 {
            format!("{} {}", bytes as u64, units[i])
        } else {
            format!("{:.1} {}", bytes, units[i])
        }
    }

    fn scan_files(&self) {
        let f1 = self.txt_folder1.text();
        let f2 = self.txt_folder2.text();

        if f1.is_empty() {
            nwg::modal_error_message(&self.window, "خطأ", "يرجى اختيار المجلد المرجع أولاً.");
            return;
        }

        self.list_view.clear();
        self.row_map.borrow_mut().clear();
        
        if self.list_view.column_len() == 0 {
            self.list_view.insert_column(nwg::InsertListViewColumn { index: Some(0), fmt: Some(nwg::ListViewColumnFlags::LEFT), width: Some(600), text: Some("الملف".to_string()) });
            self.list_view.insert_column(nwg::InsertListViewColumn { index: Some(1), fmt: Some(nwg::ListViewColumnFlags::LEFT), width: Some(150), text: Some("الإجراء".to_string()) });
            self.list_view.insert_column(nwg::InsertListViewColumn { index: Some(2), fmt: Some(nwg::ListViewColumnFlags::LEFT), width: Some(100), text: Some("الحجم".to_string()) });
        }

        self.lbl_status.set_text("جارٍ الفحص وحساب البصمات (xxHash)... يرجى الانتظار...");
        
        let is_single = f2.is_empty();
        *self.is_single.borrow_mut() = is_single;
        *self.base_folder.borrow_mut() = if is_single { f1.clone() } else { f2.clone() };

        let mut new_groups = Vec::new();

        if is_single {
            let mut by_size: HashMap<u64, Vec<(PathBuf, std::time::SystemTime)>> = HashMap::new();
            for entry in WalkDir::new(&f1).into_iter().filter_map(|e| e.ok()) {
                if entry.file_type().is_file() {
                    if let Ok(metadata) = entry.metadata() {
                        let size = metadata.len();
                        if size > 0 {
                            by_size.entry(size).or_default().push((entry.into_path(), metadata.modified().unwrap_or(std::time::SystemTime::UNIX_EPOCH)));
                        }
                    }
                }
            }

            let mut by_hash: HashMap<String, Vec<(PathBuf, std::time::SystemTime)>> = HashMap::new();
            for (_, items) in by_size.into_iter().filter(|(_, v)| v.len() > 1) {
                for (path, mtime) in items {
                    if let Some(hash) = Self::hash_file(&path) {
                        by_hash.entry(hash).or_default().push((path, mtime));
                    }
                }
            }

            for (digest, mut items) in by_hash.into_iter().filter(|(_, v)| v.len() > 1) {
                items.sort_by(|a, b| a.1.cmp(&b.1));
                let size = items[0].0.metadata().map(|m| m.len()).unwrap_or(0);
                
                let mut move_files = Vec::new();
                for item in items.iter().skip(1) {
                    move_files.push(item.0.clone());
                }
                move_files.sort();

                new_groups.push(FileGroup { digest, size, keep: items[0].0.clone(), move_files, locked: false });
            }
        } else {
            let mut sizes2 = HashSet::new();
            for entry in WalkDir::new(&f2).into_iter().filter_map(|e| e.ok()) {
                if entry.file_type().is_file() {
                    if let Ok(metadata) = entry.metadata() {
                        let size = metadata.len();
                        if size > 0 { sizes2.insert(size); }
                    }
                }
            }

            let mut ref_by_digest = HashMap::new();
            for entry in WalkDir::new(&f1).into_iter().filter_map(|e| e.ok()) {
                if entry.file_type().is_file() {
                    if let Ok(metadata) = entry.metadata() {
                        let size = metadata.len();
                        if size > 0 && sizes2.contains(&size) {
                            if let Some(hash) = Self::hash_file(entry.path()) {
                                ref_by_digest.insert(hash, entry.into_path());
                            }
                        }
                    }
                }
            }

            let mut found: HashMap<String, Vec<PathBuf>> = HashMap::new();
            for entry in WalkDir::new(&f2).into_iter().filter_map(|e| e.ok()) {
                if entry.file_type().is_file() {
                    if let Ok(metadata) = entry.metadata() {
                        let size = metadata.len();
                        if size > 0 {
                            if let Some(hash) = Self::hash_file(entry.path()) {
                                if ref_by_digest.contains_key(&hash) {
                                    found.entry(hash).or_default().push(entry.into_path());
                                }
                            }
                        }
                    }
                }
            }

            for (digest, mut move_files) in found {
                let keep = ref_by_digest.get(&digest).unwrap().clone();
                let size = keep.metadata().map(|m| m.len()).unwrap_or(0);
                move_files.sort();
                new_groups.push(FileGroup { digest, size, keep, move_files, locked: true });
            }
        }

        new_groups.sort_by(|a, b| (b.size * b.move_files.len() as u64).cmp(&(a.size * a.move_files.len() as u64)));
        
        *self.groups.borrow_mut() = new_groups;
        self.refresh_list();
        self.lbl_status.set_text(&format!("اكتمل الفحص: عُثر على {} مجموعة مكررة.", self.groups.borrow().len()));
    }

    fn refresh_list(&self) {
        self.list_view.clear();
        self.row_map.borrow_mut().clear();
        let mut row_idx: i32 = 0;
        let groups = self.groups.borrow();

        for (gi, g) in groups.iter().enumerate() {
            let title = format!("مجموعة {} — {} نسخ متطابقة", gi + 1, g.move_files.len() + 1);
            let size_str = Self::format_size(g.size as f64);
            
            self.list_view.insert_item(nwg::InsertListViewItem { index: Some(row_idx), column_index: 0, text: Some(title), image: None });
            self.list_view.update_item(row_idx as usize, nwg::InsertListViewItem { index: Some(row_idx), column_index: 2, text: Some(size_str.clone()), image: None });
            row_idx += 1;

            let keep_label = if g.locked { "✔ يُحتفظ به (المرجع)" } else { "✔ يُحتفظ به" };
            self.list_view.insert_item(nwg::InsertListViewItem { index: Some(row_idx), column_index: 0, text: Some(format!("    {}", g.keep.to_string_lossy())), image: None });
            self.list_view.update_item(row_idx as usize, nwg::InsertListViewItem { index: Some(row_idx), column_index: 1, text: Some(keep_label.to_string()), image: None });
            self.list_view.update_item(row_idx as usize, nwg::InsertListViewItem { index: Some(row_idx), column_index: 2, text: Some(size_str.clone()), image: None });
            self.row_map.borrow_mut().insert(row_idx as usize, (gi, true));
            row_idx += 1;

            for _m in &g.move_files {
                self.list_view.insert_item(nwg::InsertListViewItem { index: Some(row_idx), column_index: 0, text: Some(format!("    {}", _m.to_string_lossy())), image: None });
                self.list_view.update_item(row_idx as usize, nwg::InsertListViewItem { index: Some(row_idx), column_index: 1, text: Some("↪ سيُنقل".to_string()), image: None });
                self.list_view.update_item(row_idx as usize, nwg::InsertListViewItem { index: Some(row_idx), column_index: 2, text: Some(size_str.clone()), image: None });
                self.row_map.borrow_mut().insert(row_idx as usize, (gi, false));
                row_idx += 1;
            }
        }
    }

    fn clear_results(&self) {
        self.groups.borrow_mut().clear();
        self.row_map.borrow_mut().clear();
        self.list_view.clear();
        self.lbl_status.set_text("تم مسح النتائج.");
    }

    fn make_original(&self) {
        if let Some(idx) = self.list_view.selected_item() {
            let map = self.row_map.borrow();
            if let Some(&(gi, is_keep)) = map.get(&idx) {
                if is_keep { return; }
                let mut groups = self.groups.borrow_mut();
                if groups[gi].locked {
                    nwg::modal_info_message(&self.window, "تنبيه", "في وضع المجلدين، الأصل هو دائماً ملف المجلد المرجع.");
                    return;
                }
                
                let text = self.list_view.item(idx, 0, 1024).unwrap().text.trim().to_string();
                let target_path = PathBuf::from(text);
                
                let mut m_idx = None;
                for (i, p) in groups[gi].move_files.iter().enumerate() {
                    if p == &target_path { m_idx = Some(i); break; }
                }
                
                if let Some(i) = m_idx {
                    let old_keep = groups[gi].keep.clone();
                    groups[gi].keep = groups[gi].move_files.remove(i);
                    groups[gi].move_files.push(old_keep);
                    groups[gi].move_files.sort();
                }
                drop(groups);
                drop(map);
                self.refresh_list();
            }
        } else {
            nwg::modal_info_message(&self.window, "تنبيه", "اختر ملفاً من القائمة أولاً.");
        }
    }

    fn preview_file(&self) {
        if let Some(idx) = self.list_view.selected_item() {
            let map = self.row_map.borrow();
            if let Some(&(_gi, _is_keep)) = map.get(&idx) {
                let text = self.list_view.item(idx, 0, 1024).unwrap().text.trim().to_string();
                let _ = opener::open(text);
            }
        } else {
            nwg::modal_info_message(&self.window, "تنبيه", "اختر ملفاً لمعاينته.");
        }
    }

    fn quarantine_files(&self) {
        let groups = self.groups.borrow();
        if groups.is_empty() { return; }

        let base = PathBuf::from(self.base_folder.borrow().clone());
        let timestamp = Local::now().format("%Y%m%d_%H%M%S").to_string();
        let quarantine = base.join(format!("_quarantine_{}", timestamp));

        let mut tasks = Vec::new();
        for g in groups.iter() {
            for m in &g.move_files {
                tasks.push(m.clone());
            }
        }

        if tasks.is_empty() { return; }

        let msg = format!("سيتم نقل {} ملف إلى الحجر في:\n{}\n\nهل أنت متأكد؟", tasks.len(), quarantine.to_string_lossy());
        let choice = nwg::modal_message(&self.window, &nwg::MessageParams {
            title: "تأكيد النقل",
            content: &msg,
            buttons: nwg::MessageButtons::YesNo,
            icons: nwg::MessageIcons::Question
        });

        if choice == nwg::MessageChoice::Yes {
            std::fs::create_dir_all(&quarantine).unwrap_or_default();
            let mut moved = 0;
            for file in tasks {
                if let Ok(rel) = file.strip_prefix(&base) {
                    let dest = quarantine.join(rel);
                    if let Some(p) = dest.parent() {
                        std::fs::create_dir_all(p).unwrap_or_default();
                    }
                    if std::fs::rename(&file, &dest).is_ok() {
                        moved += 1;
                    }
                }
            }
            
            nwg::modal_info_message(&self.window, "اكتمل النقل", &format!("تم نقل {} ملف بنجاح.", moved));
            drop(groups);
            self.clear_results();
        }
    }

    // --- Smart Priority Feature ---
    fn open_smart_window(&self) {
        if *self.is_single.borrow() == false || self.groups.borrow().is_empty() {
            nwg::modal_info_message(&self.window, "تنبيه", "تعمل هذه الميزة في وضع المجلد الواحد فقط (عند وجود مكررات).");
            return;
        }

        let mut counts: HashMap<String, usize> = HashMap::new();
        let base = PathBuf::from(self.base_folder.borrow().clone());
        
        for g in self.groups.borrow().iter() {
            let mut seen = HashSet::new();
            let mut all = vec![g.keep.clone()];
            all.extend(g.move_files.clone());
            
            for p in all {
                if let Ok(rel) = p.strip_prefix(&base) {
                    let top = rel.components().next().map(|c| c.as_os_str().to_string_lossy().to_string()).unwrap_or_else(|| "<المجلد الرئيسي>".to_string());
                    if seen.insert(top.clone()) {
                        *counts.entry(top).or_insert(0) += 1;
                    }
                }
            }
        }

        let mut sorted: Vec<_> = counts.into_iter().collect();
        sorted.sort_by(|a, b| b.1.cmp(&a.1));

        let mut items = Vec::new();
        let mut map = HashMap::new();
        for (folder, count) in sorted {
            let display = format!("المجلد: [{}] ← (في {} مجموعة)", folder, count);
            items.push(display.clone());
            map.insert(display, folder);
        }

        *self.smart_folder_map.borrow_mut() = map;
        self.smart_list.set_collection(items);
        self.smart_window.set_visible(true);
    }

    fn smart_up(&self) {
        if let Some(idx) = self.smart_list.selection() {
            if idx > 0 {
                let mut col = self.smart_list.collection().clone();
                col.swap(idx, idx - 1);
                self.smart_list.set_collection(col);
                self.smart_list.set_selection(Some(idx - 1));
            }
        }
    }

    fn smart_down(&self) {
        if let Some(idx) = self.smart_list.selection() {
            let mut col = self.smart_list.collection().clone();
            if idx < col.len() - 1 {
                col.swap(idx, idx + 1);
                self.smart_list.set_collection(col);
                self.smart_list.set_selection(Some(idx + 1));
            }
        }
    }

    fn smart_apply(&self) {
        let col = self.smart_list.collection();
        let map = self.smart_folder_map.borrow();
        let ordered_folders: Vec<String> = col.iter().filter_map(|display| map.get(display).cloned()).collect();
        
        let base = PathBuf::from(self.base_folder.borrow().clone());
        let mut changed = 0;
        let mut groups = self.groups.borrow_mut();

        for g in groups.iter_mut() {
            let mut all = vec![g.keep.clone()];
            all.extend(g.move_files.clone());
            
            let mut preferred = None;
            for pref in &ordered_folders {
                for p in &all {
                    if let Ok(rel) = p.strip_prefix(&base) {
                        let top = rel.components().next().map(|c| c.as_os_str().to_string_lossy().to_string()).unwrap_or_else(|| "<المجلد الرئيسي>".to_string());
                        if &top == pref {
                            preferred = Some(p.clone());
                            break;
                        }
                    }
                }
                if preferred.is_some() { break; }
            }

            if let Some(pref) = preferred {
                if pref != g.keep {
                    g.move_files.retain(|x| x != &pref);
                    g.move_files.push(g.keep.clone());
                    g.keep = pref;
                    g.move_files.sort();
                    changed += 1;
                }
            }
        }

        drop(groups);
        drop(map);
        self.smart_window.set_visible(false);
        self.refresh_list();
        nwg::modal_info_message(&self.window, "نجاح", &format!("تم تغيير الأصل في {} مجموعة.", changed));
    }
}

fn main() {
    nwg::init().expect("Failed to init NWG");
    nwg::Font::set_global_family("Segoe UI").expect("Failed to set default font");
    
    let _app = DuplicateCleaner::build_ui(Default::default()).expect("Failed to build UI");
    
    nwg::dispatch_thread_events();
}