use std::io::{Read, Write};
use std::net::{TcpListener, TcpStream};

// ตั้ง buffer size ให้เล็กเพื่อให้เห็นการแบ่ง boundary ชัดเจน
const BUFFER_SIZE: usize = 64;

fn print_separator() {
    println!("{}", "=".repeat(80));
}

fn print_chunk_header(chunk_num: usize, size: usize, progress: usize, total: usize) {
    println!("\n📦 CHUNK #{} ({} bytes) - Progress: {}/{} bytes", 
             chunk_num, size, progress, total);
    println!("{}", "-".repeat(80));
}

fn visualize_boundary(data: &[u8], boundary_pattern: &str) {
    let text = String::from_utf8_lossy(data);
    
    // แสดงข้อมูลดิบ
    println!("Raw data:");
    println!("{}", text);
    
    // หา boundary positions
    if let Some(pos) = text.find(boundary_pattern) {
        println!("\n🎯 FOUND COMPLETE BOUNDARY at position {}", pos);
        
        // แสดงก่อนและหลัง boundary
        let before = &text[..pos];
        let boundary = &text[pos..pos + boundary_pattern.len()];
        let after = if pos + boundary_pattern.len() < text.len() {
            &text[pos + boundary_pattern.len()..]
        } else {
            ""
        };
        
        println!("  Before: {:?}", before);
        println!("  Boundary: {:?}", boundary);
        println!("  After: {:?}", after);
    }
    
    // ตรวจสอบ partial boundary ที่ท้าย chunk
    let boundary_bytes = boundary_pattern.as_bytes();
    for i in 1..boundary_bytes.len() {
        if data.len() >= i && &data[data.len() - i..] == &boundary_bytes[..i] {
            println!("\n⚠️  PARTIAL BOUNDARY at end ({} bytes): {:?}", 
                     i, 
                     String::from_utf8_lossy(&data[data.len() - i..]));
            println!("   This might continue in next chunk!");
            break; // แสดงแค่ partial ที่ยาวที่สุด
        }
    }
}

fn handle_client(mut stream: TcpStream) {
    print_separator();
    println!("🔌 NEW CLIENT CONNECTED");
    print_separator();

    let mut buffer = [0u8; BUFFER_SIZE];
    let mut chunk_num = 0;
    let mut total_bytes = 0;
    let mut all_data = Vec::new();

    // อ่าน HTTP headers ก่อนเพื่อหา boundary
    let mut header_buffer = Vec::new();
    let mut temp_buf = [0u8; 1];
    let mut found_boundary = String::new();
    let mut content_length = 0usize;
    
    // อ่าน headers จนเจอ \r\n\r\n
    loop {
        if stream.read(&mut temp_buf).unwrap_or(0) == 0 {
            break;
        }
        header_buffer.push(temp_buf[0]);
        
        if header_buffer.len() >= 4 
            && &header_buffer[header_buffer.len()-4..] == b"\r\n\r\n" {
            break;
        }
    }
    
    // Parse boundary และ Content-Length จาก headers
    let headers = String::from_utf8_lossy(&header_buffer);
    println!("📋 HTTP HEADERS:");
    println!("{}", headers);
    print_separator();
    
    // หา boundary
    if let Some(content_type_line) = headers.lines()
        .find(|l| l.to_lowercase().starts_with("content-type:")) {
        if let Some(boundary_start) = content_type_line.find("boundary=") {
            found_boundary = content_type_line[boundary_start + 9..]
                .trim()
                .to_string();
            found_boundary = format!("--{}", found_boundary);
            println!("🔍 Detected boundary: {:?}", found_boundary);
        }
    }
    
    // หา Content-Length
    if let Some(cl_line) = headers.lines()
        .find(|l| l.to_lowercase().starts_with("content-length:")) {
        if let Some(len_str) = cl_line.split(':').nth(1) {
            content_length = len_str.trim().parse().unwrap_or(0);
            println!("📏 Content-Length: {} bytes", content_length);
        }
    }
    
    print_separator();

    // อ่าน body ตาม Content-Length
    let mut bytes_read = 0;
    let final_boundary = format!("{}--", found_boundary);
    let mut found_end = false;
    
    loop {
        // ถ้ามี Content-Length ให้ใช้เป็นตัวกำหนด
        if content_length > 0 && bytes_read >= content_length {
            println!("\n✅ READ COMPLETE ({}/{} bytes)", bytes_read, content_length);
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
                println!("\n🔚 CONNECTION CLOSED"); //ส่วนใหญ่ตอนนี้ใช้ http 1.1 ทำให้เกิด keep alive แปลว่า ต่อให้ส่งข้อมูลครบแล้วก็จะไม่่ปิด  connterction tcp
                break;
            }
            Ok(n) => {
                chunk_num += 1;
                bytes_read += n;
                total_bytes += n;

                let progress_total = if content_length > 0 { content_length } else { bytes_read };
                print_chunk_header(chunk_num, n, bytes_read, progress_total);
                
                let chunk_data = &buffer[..n];
                all_data.extend_from_slice(chunk_data);
                
                // แสดง chunk ในรูปแบบที่อ่านง่าย
                visualize_boundary(chunk_data, &found_boundary);
                
                // แสดง hex ของ 20 bytes แรกและท้าย
                println!("\nFirst 20 bytes (hex): {:02x?}", 
                         &chunk_data[..n.min(20)]);
                if n > 20 {
                    println!("Last 20 bytes (hex): {:02x?}", 
                             &chunk_data[n.saturating_sub(20)..]);
                }
                
                // ตรวจสอบ final boundary (สำหรับกรณีไม่มี Content-Length)
                let chunk_str = String::from_utf8_lossy(chunk_data);
                if chunk_str.contains(&final_boundary) {
                    found_end = true;
                    println!("\n🏁 FOUND FINAL BOUNDARY");
                    if content_length == 0 {
                        break;
                    }
                }
            }
            Err(e) => {
                eprintln!("❌ Error reading: {}", e);
                break;
            }
        }
    }

    print_separator();
    println!("📊 SUMMARY");
    println!("Total chunks: {}", chunk_num);
    println!("Total bytes: {}", total_bytes);
    
    // นับจำนวน boundary ทั้งหมด
    let full_data = String::from_utf8_lossy(&all_data);
    let boundary_count = full_data.matches(&found_boundary).count();
    println!("Total boundaries found: {}", boundary_count);
    
    if found_end {
        println!("✅ Found final boundary: {}", final_boundary);
    }
    
    print_separator();

    // ส่ง response กลับ
    let response = "HTTP/1.1 200 OK\r\nContent-Length: 2\r\n\r\nOK";
    match stream.write_all(response.as_bytes()) {
        Ok(_) => println!("✅ Response sent successfully"),
        Err(e) => eprintln!("❌ Error sending response: {}", e),
    }
}

fn main() {
    let listener = TcpListener::bind("127.0.0.1:8080")
        .expect("Failed to bind to address");

    println!("\n🚀 SERVER STARTED");
    println!("📍 Listening on: 127.0.0.1:8080");
    println!("📦 Buffer size: {} bytes (intentionally small to split boundaries)", BUFFER_SIZE);
    println!("\n💡 Test with curl:");
    println!("   curl -X POST http://127.0.0.1:8080/upload \\");
    println!("        -F \"field1=value1\" \\");
    println!("        -F \"field2=value2\" \\");
    println!("        -F \"username=JohnDoe\" \\");
    println!("        -F \"email=john.doe@example.com\"");
    println!("\n   Or with a file:");
    println!("   curl -X POST http://127.0.0.1:8080/upload \\");
    println!("        -F \"username=JohnDoe\" \\");
    println!("        -F \"profile=@/path/to/file.jpg\"");
    print_separator();

    for stream in listener.incoming() {
        match stream {
            Ok(stream) => {
                handle_client(stream);
            }
            Err(e) => {
                eprintln!("❌ Error accepting connection: {}", e);
            }
        }
    }
}