use std::fs::{File, create_dir_all};
use std::io::{Read, Write, BufWriter};
use std::net::{TcpListener, TcpStream};
use std::time::Instant;

const BUFFER_SIZE: usize = 8192; // 8KB buffer สำหรับ streaming
const UPLOAD_DIR: &str = "./uploads";

#[allow(dead_code)]
#[derive(Debug)]
enum PartType {
    Field,
    File { filename: String, content_type: String },
}


struct StreamingParser {
    boundary: Vec<u8>,
    retained: Vec<u8>,
    state: ParserState,
    current_part_type: Option<PartType>,
    current_field_name: String,
    file_writer: Option<BufWriter<File>>,
    stats: Stats,
}

#[derive(Debug, PartialEq)]
enum ParserState {
    SearchingBoundary,
    ReadingHeaders,
    ReadingData,
}

#[derive(Debug)]
struct Stats {
    total_chunks: usize,
    total_bytes: usize,
    fields_count: usize,
    files_count: usize,
    files_saved: Vec<FileInfo>,
}

#[derive(Debug, Clone)]
struct FileInfo {
    field_name: String,
    filename: String,
    size: usize,
    path: String,
}

impl StreamingParser {
    fn new(boundary: &str) -> Self {
        create_dir_all(UPLOAD_DIR).ok();
        
        Self {
            boundary: boundary.as_bytes().to_vec(),
            retained: Vec::new(),
            state: ParserState::SearchingBoundary,
            current_part_type: None,
            current_field_name: String::new(),
            file_writer: None,
            stats: Stats {
                total_chunks: 0,
                total_bytes: 0,
                fields_count: 0,
                files_count: 0,
                files_saved: Vec::new(),
            },
        }
    }

    fn process_chunk(&mut self, chunk: &[u8]) {
        self.stats.total_chunks += 1;
        self.stats.total_bytes += chunk.len();

        // รวม retained + chunk ใหม่
        let mut data = self.retained.clone();
        data.extend_from_slice(chunk);

        let mut pos = 0;

        while pos < data.len() {
            match self.state {
                ParserState::SearchingBoundary => {
                    // หา boundary
                    if let Some(boundary_pos) = self.find_boundary(&data[pos..]) {
                        let actual_pos = pos + boundary_pos;
                        
                        // ข้าม boundary
                        pos = actual_pos + self.boundary.len();
                        
                        // ข้าม \r\n หลัง boundary
                        if pos + 2 <= data.len() && &data[pos..pos + 2] == b"\r\n" {
                            pos += 2;
                        }
                        
                        self.state = ParserState::ReadingHeaders;
                        self.current_field_name.clear();
                        self.current_part_type = None;
                    } else {
                        // ยังไม่เจอ boundary, retain ท้าย buffer
                        break;
                    }
                }
                
                ParserState::ReadingHeaders => {
                    // หา \r\n\r\n (จบ headers)
                    if let Some(header_end) = self.find_pattern(&data[pos..], b"\r\n\r\n") {
                        let headers_data = &data[pos..pos + header_end];
                        let headers = String::from_utf8_lossy(headers_data);
                        
                        self.parse_headers(&headers);
                        
                        pos += header_end + 4; // ข้าม \r\n\r\n
                        self.state = ParserState::ReadingData;
                        
                        // ถ้าเป็นไฟล์ ให้เปิด file writer
                        // แก้ไข: clone filename ก่อนเพื่อหลีกเลี่ยง borrow conflict
                        if let Some(PartType::File { filename, .. }) = &self.current_part_type {
                            let filename_clone = filename.clone();
                            self.open_file_writer(&filename_clone);
                        }
                    } else {
                        // ยังอ่าน headers ไม่ครบ
                        break;
                    }
                }
                
                ParserState::ReadingData => {
                    // หา boundary ถัดไป
                    if let Some(boundary_pos) = self.find_boundary(&data[pos..]) {
                        let data_chunk = &data[pos..pos + boundary_pos - 2]; // -2 เพื่อข้าม \r\n ก่อน boundary
                        
                        // เขียนข้อมูลส่วนสุดท้าย
                        self.write_data(data_chunk);
                        
                        // ปิดไฟล์ถ้ามี
                        self.close_file_writer();
                        
                        pos += boundary_pos;
                        self.state = ParserState::SearchingBoundary;
                    } else {
                        // ยังไม่เจอ boundary ถัดไป, เขียนข้อมูลที่มี (ยกเว้นท้าย buffer)
                        let safe_write_len = data.len().saturating_sub(pos + self.boundary.len());
                        
                        if safe_write_len > 0 {
                            self.write_data(&data[pos..pos + safe_write_len]);
                            pos += safe_write_len;
                        }
                        break;
                    }
                }
            }
        }

        // Retain ข้อมูลที่เหลือ
        if pos < data.len() {
            self.retained = data[pos..].to_vec();
        } else {
            self.retained.clear();
        }
    }

    fn find_boundary(&self, data: &[u8]) -> Option<usize> {
        for i in 0..=data.len().saturating_sub(self.boundary.len()) {
            if &data[i..i + self.boundary.len()] == self.boundary.as_slice() {
                return Some(i);
            }
        }
        None
    }

    fn find_pattern(&self, data: &[u8], pattern: &[u8]) -> Option<usize> {
        for i in 0..=data.len().saturating_sub(pattern.len()) {
            if &data[i..i + pattern.len()] == pattern {
                return Some(i);
            }
        }
        None
    }

    fn parse_headers(&mut self, headers: &str) {
        for line in headers.lines() {
            if line.to_lowercase().starts_with("content-disposition:") {
                // Parse field name
                if let Some(name_start) = line.find("name=\"") {
                    let name_part = &line[name_start + 6..];
                    if let Some(name_end) = name_part.find('"') {
                        self.current_field_name = name_part[..name_end].to_string();
                    }
                }
                
                // Parse filename
                if let Some(filename_start) = line.find("filename=\"") {
                    let filename_part = &line[filename_start + 10..];
                    if let Some(filename_end) = filename_part.find('"') {
                        let filename = filename_part[..filename_end].to_string();
                        self.current_part_type = Some(PartType::File {
                            filename,
                            content_type: String::new(),
                        });
                    }
                } else {
                    self.current_part_type = Some(PartType::Field);
                }
            }
            
            if line.to_lowercase().starts_with("content-type:") {
                if let Some(PartType::File { filename, .. }) = &self.current_part_type {
                    let content_type = line[13..].trim().to_string();
                    self.current_part_type = Some(PartType::File {
                        filename: filename.clone(),
                        content_type,
                    });
                }
            }
        }
    }

    fn open_file_writer(&mut self, filename: &str) {
        self.stats.files_count += 1;
        let filepath = format!("{}/{}", UPLOAD_DIR, filename);
        
        println!("\n📁 เริ่ม stream ไฟล์: {}", filename);
        println!("   Path: {}", filepath);
        println!("   🔄 กำลังเขียนโดยตรงไป disk...");
        
        if let Ok(file) = File::create(&filepath) {
            self.file_writer = Some(BufWriter::new(file));
        }
    }

    fn write_data(&mut self, data: &[u8]) {
        if let Some(writer) = &mut self.file_writer {
            // Stream ไปที่ไฟล์โดยตรง (ไม่เก็บใน memory)
            writer.write_all(data).ok();
        } else {
            // เป็น field ธรรมดา (ไม่ใช่ไฟล์)
            self.stats.fields_count += 1;
        }
    }

    fn close_file_writer(&mut self) {
        if let Some(mut writer) = self.file_writer.take() {
            writer.flush().ok();
            
            if let Some(PartType::File { filename, content_type: _ }) = &self.current_part_type {
                let filepath = format!("{}/{}", UPLOAD_DIR, filename);
                let size = std::fs::metadata(&filepath)
                    .map(|m| m.len() as usize)
                    .unwrap_or(0);
                
                println!("   ✅ บันทึกเสร็จ: {} ({} bytes)", filename, size);
                
                self.stats.files_saved.push(FileInfo {
                    field_name: self.current_field_name.clone(),
                    filename: filename.clone(),
                    size,
                    path: filepath,
                });
            }
        }
    }

    fn finalize(&mut self) {
        // แก้ไข: clone retained ก่อนเพื่อหลีกเลี่ยง borrow conflict
        if !self.retained.is_empty() {
            let retained_data = self.retained.clone();
            self.write_data(&retained_data);
            self.retained.clear();
        }
        
        self.close_file_writer();
    }

    fn get_stats(&self) -> &Stats {
        &self.stats
    }
}

fn print_separator() {
    println!("{}", "═".repeat(80));
}

fn format_bytes(bytes: usize) -> String {
    const KB: usize = 1024;
    const MB: usize = KB * 1024;
    const GB: usize = MB * 1024;

    if bytes >= GB {
        format!("{:.2} GB", bytes as f64 / GB as f64)
    } else if bytes >= MB {
        format!("{:.2} MB", bytes as f64 / MB as f64)
    } else if bytes >= KB {
        format!("{:.2} KB", bytes as f64 / KB as f64)
    } else {
        format!("{} bytes", bytes)
    }
}

fn handle_client(mut stream: TcpStream) {
    let start_time = Instant::now();
    
    // อ่าน HTTP headers
    let mut header_buffer = Vec::new();
    let mut temp_buf = [0u8; 1];
    
    loop {
        match stream.read(&mut temp_buf) {
            Ok(0) | Err(_) => break,
            Ok(_) => {
                header_buffer.push(temp_buf[0]);
                if header_buffer.len() >= 4 
                    && &header_buffer[header_buffer.len()-4..] == b"\r\n\r\n" {
                    break;
                }
            }
        }
    }
    
    let headers = String::from_utf8_lossy(&header_buffer);
    
    // Parse boundary
    let mut boundary = String::new();
    if let Some(content_type) = headers.lines()
        .find(|l| l.to_lowercase().starts_with("content-type:")) {
        if let Some(idx) = content_type.find("boundary=") {
            boundary = content_type[idx + 9..].trim().to_string();
            boundary = format!("--{}", boundary);
        }
    }

    // Parse Content-Length
    let mut content_length = 0usize;
    if let Some(cl_line) = headers.lines()
        .find(|l| l.to_lowercase().starts_with("content-length:")) {
        if let Some(len_str) = cl_line.split(':').nth(1) {
            content_length = len_str.trim().parse().unwrap_or(0);
        }
    }

    println!("\n📋 Request:");
    if let Some(first_line) = headers.lines().next() {
        println!("   {}", first_line);
    }
    println!("   Boundary: {:?}", boundary);
    println!("   Content-Length: {} ({})", content_length, format_bytes(content_length));
    println!("   Buffer: {} bytes", BUFFER_SIZE);

    print_separator();

    let mut parser = StreamingParser::new(&boundary);
    let mut buffer = [0u8; BUFFER_SIZE];
    let mut bytes_read = 0usize;
    let mut last_progress = 0;

    loop {
        // หยุดเมื่ออ่านครบ
        if content_length > 0 && bytes_read >= content_length {
            println!("\n✅ Read complete: {}/{} bytes", bytes_read, content_length);
            break;
        }

        // คำนวณว่าจะอ่านกี่ bytes
        let to_read = if content_length > 0 {
            (content_length - bytes_read).min(BUFFER_SIZE)
        } else {
            BUFFER_SIZE
        };

        match stream.read(&mut buffer[..to_read]) {
            Ok(0) => {
                println!("\n⚠️  Connection closed early: {}/{} bytes", 
                         bytes_read, content_length);
                break;
            }
            Ok(n) => {
                bytes_read += n;
                parser.process_chunk(&buffer[..n]);
                
                // คำนวณ progress
                let progress_pct = if content_length > 0 {
                    (bytes_read as f64 / content_length as f64 * 100.0) as usize
                } else {
                    0
                };

                let current_mb = bytes_read / (1024 * 1024);
                
                // แสดง progress ทุก 10% หรือทุก 10MB (แล้วแต่อันไหนเกิดก่อน)
                let show_by_percent = progress_pct > 0 && progress_pct % 10 == 0 && progress_pct != last_progress;
                let show_by_size = current_mb > 0 && current_mb % 10 == 0 && current_mb != last_progress;
                
                if show_by_percent || (show_by_size && progress_pct == 0) {
                    if content_length > 0 {
                        println!("📊 Progress: {} / {} ({}%)", 
                                 format_bytes(bytes_read),
                                 format_bytes(content_length),
                                 progress_pct);
                    } else {
                        println!("📊 Progress: {} received", format_bytes(bytes_read));
                    }
                    last_progress = if progress_pct > 0 { progress_pct } else { current_mb };
                }
            }
            Err(e) => {
                eprintln!("❌ Error: {}", e);
                break;
            }
        }
    }

    parser.finalize();

    let elapsed = start_time.elapsed();
    let stats = parser.get_stats();

    print_separator();
    println!("📊 สรุปผลลัพธ์");
    print_separator();

    println!("\n⏱️  เวลาที่ใช้: {:.2?}", elapsed);
    println!("\n📦 ข้อมูลที่รับ:");
    println!("   Total chunks: {}", stats.total_chunks);
    println!("   Total bytes: {} ({})", stats.total_bytes, format_bytes(stats.total_bytes));
    println!("   Fields: {}", stats.fields_count);
    println!("   Files: {}", stats.files_count);

    if stats.total_bytes > 0 && elapsed.as_secs_f64() > 0.0 {
        let speed = stats.total_bytes as f64 / elapsed.as_secs_f64();
        println!("\n⚡ ความเร็ว: {}/sec", format_bytes(speed as usize));
    }

    if !stats.files_saved.is_empty() {
        println!("\n📁 ไฟล์ที่บันทึก:");
        println!("{}", "─".repeat(80));
        
        for file in &stats.files_saved {
            println!("\n✅ {}", file.filename);
            println!("   Field: {}", file.field_name);
            println!("   Size: {} ({})", file.size, format_bytes(file.size));
            println!("   Path: {}", file.path);
        }
    }

    println!("\n{}", "─".repeat(80));
    println!("💾 ไฟล์ทั้งหมดถูก stream ไปยัง disk โดยตรง");
    println!("🚀 ไม่มี memory overhead ไม่ว่าไฟล์จะใหญ่แค่ไหน!");
    print_separator();

    // ส่ง response
    let response = "HTTP/1.1 200 OK\r\nContent-Length: 2\r\n\r\nOK";
    match stream.write_all(response.as_bytes()) {
        Ok(_) => println!("✅ Response sent successfully"),
        Err(e) => eprintln!("❌ Error sending response: {}", e),
    }
}

fn main() {

    println!("\n📍 Server: 127.0.0.1:8082");
    println!("📦 Stream Buffer: {} bytes", BUFFER_SIZE);
    println!("💾 Upload Directory: {}", UPLOAD_DIR);
    println!("🎯 วัตถุประสงค์: รับไฟล์ขนาดใหญ่โดย stream ไป disk โดยตรง");
    
    println!("\n💡 สร้างไฟล์ทดสอบ 1GB:");
    println!("   # Linux/Mac");
    println!("   dd if=/dev/zero bs=1m count=1024 | tr '\0' 'a' > file.txt");

    
    println!("\n💡 ทดสอบด้วย curl:");
    println!("   curl -X POST http://127.0.0.1:8082/upload \\");
    println!("        -F \"username=JohnDoe\" \\");
    println!("        -F \"file=@file.txt\" \\");
    println!("        -F \"description=Large file test\"");
    

    print_separator();

    let listener = TcpListener::bind("127.0.0.1:8082")
        .expect("Cannot bind to port 8082");

    println!("\n⏳ Waiting for connections...\n");

    for stream in listener.incoming() {
        match stream {
            Ok(stream) => {
                handle_client(stream);
                println!("\n⏳ Waiting for next connection...\n");
            }
            Err(e) => eprintln!("❌ Error: {}", e),
        }
    }
}