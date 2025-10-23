# sub_lab1
repo นี้จะแสดงให้เห็นการจัด ทำ http multipart boundary ด้วยเข้าใจโครงสร้างของมัน

### แสดงให้เห็นว่า Boundary ขาดได้

- แสดงแต่ละ chunk ที่อ่านมา
- ชี้ให้เห็นว่า boundary ขาดตรงไหน
- แสดง partial boundary ที่ท้าย chunk

## คำอธิบาย HTTP Multipart Boundary
🎯 Multipart คืออะไร?
Multipart/form-data เป็นวิธีส่งข้อมูลหลายส่วน (หลาย fields) ในคำขอ HTTP เดียว โดยแต่ละส่วนคั่นด้วย boundary
ใช้เวลา:

- Upload ไฟล์
- ส่งฟอร์มที่มีทั้งข้อความและไฟล์
- ส่งข้อมูลหลาย field พร้อมกัน


### โครงสร้าง Multipart Request

```

┌─────────────────────────────┐
│   HTTP Request              │
├─────────────────────────────┤
│ Headers                     │
│ Content-Type: multipart/... │
├─────────────────────────────┤
│ Body:                       │
│   Part 1: username (text)   │
│   ─────── boundary ─────    │
│   Part 2: email (text)      │
│   ─────── boundary ─────    │
│   Part 3: avatar (file)     │
│   ─────── boundary ─────    │
└─────────────────────────────┘

```


### example of request

```
POST /upload HTTP/1.1
Host: example.com
Content-Type: multipart/form-data; boundary=----WebKitFormBoundary7MA4YWxkTrZu0gW
Content-Length: 1234

------WebKitFormBoundary7MA4YWxkTrZu0gW
Content-Disposition: form-data; name="username"

JohnDoe
------WebKitFormBoundary7MA4YWxkTrZu0gW
Content-Disposition: form-data; name="email"

john@example.com
------WebKitFormBoundary7MA4YWxkTrZu0gW
Content-Disposition: form-data; name="avatar"; filename="photo.jpg"
Content-Type: image/jpeg

[binary image data]
------WebKitFormBoundary7MA4YWxkTrZu0gW--
```



### How to run

``` cargo r ```

then

``` 
curl -X POST http://127.0.0.1:8080/upload \
        -F "field1=value1" \
        -F "field2=value2" \
        -F "username=JohnDoe" \
        -F "email=john.doe@example.com"
 ```


> [NOTE!!]
> lab นี้ควรอ่าน lab ก่่อนหน้าของผมมาเพื่อ เข้าใจที่ดีขึ้น
> https://github.com/thodsaphon-nueng/tcp_kernel_buffer_sl_flow_control
> และแนะนำให้อ่านว่า http 1.0 / 1.1 ต่างกันยังไง HINT (keep alive)