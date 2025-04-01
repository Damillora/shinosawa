use std::{
    collections::HashMap, fs::{self, File}, io::{self, Seek}
};

use fatfs::Dir;

pub const IMAGE_FILE: &str = "target/shinosawa.img";

const TARGET: &str = "x86_64-shinosawa";
const EFI_ROOT: &str = "efi_root";
const FAT_FILE: &str = "target/shinosawa-rootfs.img";

const PART_SIZE: u64 = 100 * 1024 * 1024; // 100 MB
const DISK_SIZE: u64 = PART_SIZE + 1024 * 64; // for GPT headers

fn copy_efi_root(root_dir: &Dir<'_, &File>) {
    use walkdir::WalkDir;

    for entry in WalkDir::new(EFI_ROOT).min_depth(1).into_iter().filter_map(|e| e.ok()) {
        let entry_path = entry.path().strip_prefix(EFI_ROOT).unwrap().to_str().unwrap();
        let actual_path = entry.path().to_str().unwrap();
        if entry.metadata().unwrap().is_dir() {
            println!("rootfs: mkdir {}", entry_path);
            root_dir.create_dir(entry_path).unwrap();
        } else if entry.metadata().unwrap().is_file() {
            println!("rootfs: cp {} {}", actual_path, entry_path);
            let mut file = root_dir.create_file(entry_path).unwrap();
            file.truncate().unwrap();
            io::copy(&mut fs::File::open(actual_path).unwrap(), &mut file).unwrap();
        }
    }
}

fn create_shinosawa_layout(root_dir: &Dir<'_, &File>) {
    root_dir.create_dir("shinosawa").unwrap();
    root_dir.create_dir("shinosawa/system").unwrap();
}

fn copy_shinosawa_system_files(root_dir: Dir<'_, &File>, entries: HashMap<String, String>) {
    for (source_file, dest_file) in entries {
        println!("rootfs: cp {} {}", source_file, dest_file);
        let mut file = root_dir.create_file(&dest_file).unwrap();
        file.truncate().unwrap();
        io::copy(&mut fs::File::open(source_file).unwrap(), &mut file).unwrap();
    }
}

fn create_fat_image(kernel_path: String) {
    // create new filesystem image file at the given path and set its length
    let fat_file = fs::OpenOptions::new()
        .read(true)
        .write(true)
        .create(true)
        .truncate(true)
        .open(FAT_FILE)
        .unwrap();
    fat_file.set_len(PART_SIZE).unwrap();

    // create new FAT file system and open it
    let format_options = fatfs::FormatVolumeOptions::new();
    fatfs::format_volume(&fat_file, format_options).unwrap();
    let filesystem = fatfs::FileSystem::new(&fat_file, fatfs::FsOptions::new()).unwrap();


    // copy EFI file to FAT filesystem
    let root_dir = filesystem.root_dir();
    copy_efi_root(&root_dir);
    create_shinosawa_layout(&root_dir);

    let mut files = HashMap::new();
    files.insert(kernel_path, String::from("shinosawa/system/kernel"));
    copy_shinosawa_system_files(root_dir, files);
}

fn create_gpt_image() {
    // create new disk file
    let mut disk_image = fs::OpenOptions::new()
        .create(true)
        .truncate(true)
        .read(true)
        .write(true)
        .open(IMAGE_FILE)
        .unwrap();

    disk_image.set_len(DISK_SIZE).unwrap();

    // create protective MBR
    let mbr = gpt::mbr::ProtectiveMBR::with_lb_size(
        u32::try_from((DISK_SIZE / 512) - 1).unwrap_or(0xFF_FF_FF_FF),
    );
    mbr.overwrite_lba0(&mut disk_image)
        .expect("Failed to write protective MBR");

    // create new GPT structure
    let block_size = gpt::disk::LogicalBlockSize::Lb512;
    let mut gpt_disk = gpt::GptConfig::new()
        .writable(true)
        .logical_block_size(block_size)
        .create_from_device(Box::new(&mut disk_image), None)
        .expect("Failed to create GPT disk");
    gpt_disk.update_partitions(Default::default()).unwrap();

    // Get FAT partition size
    let partition_size: u64 = fs::metadata(&FAT_FILE).unwrap().len();

    // add new EFI system partition and get its byte offset in the file
    let partition_id = gpt_disk
        .add_partition(
            "SHINOSAWA",
            partition_size,
            gpt::partition_types::EFI,
            0,
            None,
        )
        .expect("Failed to create ");
    let partition = gpt_disk.partitions().get(&partition_id).unwrap();
    let start_offset = partition.bytes_start(block_size).unwrap();

    // close the GPT structure and write out changes
    gpt_disk.write().unwrap();

    // place the FAT filesystem in the newly created partition
    disk_image.seek(io::SeekFrom::Start(start_offset)).unwrap();
    io::copy(&mut File::open(&FAT_FILE).unwrap(), &mut disk_image).unwrap();
}

pub fn command(profile: String, kernel_image: Option<String>) {
    let kernel_path = match kernel_image {
        Some(str) => str,
        None => format!("target/{}/{}/kernel", TARGET, profile),
    };
    println!("using kernel {}", kernel_path);

    create_fat_image(kernel_path);
    create_gpt_image();

    // Cleanup
    match fs::remove_file(FAT_FILE) {
        Ok(_) => (),
        Err(e) => match e.kind() {
            io::ErrorKind::NotFound => (),
            _ => (),
        }
    };
}
