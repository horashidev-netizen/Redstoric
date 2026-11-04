import os
from pathlib import Path

# Đường dẫn thư mục dự án (thay đổi nếu cần)
PROJECT_DIR = Path(".")

# File đầu ra
OUTPUT_FILE = "project_dump.md"

# Các thư mục bỏ qua để tránh file quá nặng hoặc file rác build
IGNORE_DIRS = {
    ".git", "target", ".cargo", ".github", ".gitlab",
    "node_modules", ".idea", ".vscode", "__pycache__"
}

# Các file bỏ qua
IGNORE_FILES = {
    "Cargo.lock", "flake.lock", ".DS_Store", "package-lock.json", OUTPUT_FILE
}

# Các đuôi file mã nguồn/cấu hình cần đọc (bỏ qua file ảnh, âm thanh, 3D/voxel nhị phân)
VALID_EXTENSIONS = {
    ".rs", ".toml", ".md", ".json", ".yml", ".yaml",
    ".txt", ".sh", ".py", ".ron", ".wgsl", ".glsl"
}

def is_text_file(filepath: Path) -> bool:
    """Kiểm tra file có thuộc danh sách đuôi file văn bản được chọn hay không."""
    return filepath.suffix.lower() in VALID_EXTENSIONS

def generate_tree(dir_path: Path, prefix: str = "") -> list[str]:
    """Sinh cấu trúc cây thư mục."""
    tree_lines = []
    try:
        entries = sorted(
            [e for e in dir_path.iterdir() if e.name not in IGNORE_DIRS and e.name not in IGNORE_FILES],
            key=lambda x: (x.is_file(), x.name.lower())
        )
    except PermissionError:
        return []

    for index, entry in enumerate(entries):
        connector = "└── " if index == len(entries) - 1 else "├── "
        tree_lines.append(f"{prefix}{connector}{entry.name}")

        if entry.is_dir():
            new_prefix = prefix + ("    " if index == len(entries) - 1 else "│   ")
            tree_lines.extend(generate_tree(entry, new_prefix))

    return tree_lines

def dump_project(root_path: Path, output_path: Path):
    root_path = root_path.resolve()
    print(f"Đang quét thư mục: {root_path}")

    with open(output_path, "w", encoding="utf-8") as out:
        # 1. Ghi cấu trúc thư mục
        out.write(f"# Cấu trúc dự án: {root_path.name}\n\n")
        out.write("```text\n")
        out.write(f"{root_path.name}/\n")
        tree_lines = generate_tree(root_path)
        out.write("\n".join(tree_lines))
        out.write("\n```\n\n---\n\n")

        # 2. Ghi nội dung chi tiết từng file mã nguồn
        out.write("# Nội dung mã nguồn chi tiết\n\n")

        file_count = 0
        for current_root, dirs, files in os.walk(root_path):
            # Lọc bỏ các thư mục ignore trong os.walk
            dirs[:] = [d for d in dirs if d not in IGNORE_DIRS]

            for file in sorted(files):
                if file in IGNORE_FILES:
                    continue

                file_path = Path(current_root) / file
                if not is_text_file(file_path):
                    continue

                rel_path = file_path.relative_to(root_path)
                ext = file_path.suffix.lstrip(".") or "text"

                try:
                    with open(file_path, "r", encoding="utf-8") as f:
                        content = f.read()

                    out.write(f"## File: `{rel_path}`\n\n")
                    out.write(f"```{ext}\n")
                    out.write(content)
                    if not content.endswith("\n"):
                        out.write("\n")
                    out.write("```\n\n")

                    file_count += 1
                except (UnicodeDecodeError, PermissionError):
                    continue

    print(f"Hoàn thành! Đã xuất {file_count} file ra '{output_path}'.")

if __name__ == "__main__":
    dump_project(PROJECT_DIR, Path(OUTPUT_FILE))