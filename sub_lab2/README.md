# sub_lab2
repo นี้จะแสดงให้เห็นการจัดการกับ upload large file  โดยใช้ความเข้าใจที่ได้มาจาก sub lab1 น่ะครับ

## How to run


create file for uploading 

```
10mb 
dd if=/dev/zero bs=1m count=10 | tr '\0' 'a' > file10mb.txt
1Gb
dd if=/dev/zero bs=1m count=1024 | tr '\0' 'a' > file.txt

```

```
cargo r
```

then

```
curl -X POST http://127.0.0.1:8082/upload \
        -F "username=JohnDoe" \
        -F "file=@file10mb.txt" \
        -F "description=Large file test"

curl -X POST http://127.0.0.1:8082/upload \
        -F "username=JohnDoe" \
        -F "file=@file.txt" \
        -F "description=Large file test"

```


## Figures ScreenShot

### 10mb file
</br>
<img width="1464" height="602" alt="Image" src="https://github.com/user-attachments/assets/ea9089b4-4a06-4fbc-beb0-52d6068bf5d7" />
<div align="center">
  Fig A.
</div>


<img width="849" height="57" alt="Image" src="https://github.com/user-attachments/assets/6e17667e-835f-4cd5-bccc-f32a89a823ca" />
<div align="center">
  Fig B.
</div>
</br>

<img width="1155" height="96" alt="Image" src="https://github.com/user-attachments/assets/3dd010a3-f7a6-40b2-be78-17cccc8d80f2" />
<div align="center">
  Fig C.
</div>
</br>

<img width="1318" height="871" alt="Image" src="https://github.com/user-attachments/assets/7ec45baa-0901-4a73-aca0-56021dac3ca9" />
<div align="center">
  Fig D.
</div>
</br>

<img width="983" height="96" alt="Image" src="https://github.com/user-attachments/assets/c4533d65-d824-4670-89b3-4a1b491b5828" />
<div align="center">
  Fig E.
</div>
</br>


### 1Gb file
<img width="1117" height="105" alt="Image" src="https://github.com/user-attachments/assets/58dbf32d-587a-458e-bb4d-f94b308b8387" />
<div align="center">
  Fig A.
</div>
</br>

<img width="1330" height="845" alt="Image" src="https://github.com/user-attachments/assets/b98ee5aa-7e58-49d9-a662-acf3bb53665b" />
<div align="center">
  Fig B.
</div>
</br>

<img width="901" height="112" alt="Image" src="https://github.com/user-attachments/assets/ec51951b-a7b0-4f3a-a529-b06505cd6a44" />
<div align="center">
  Fig C.
</div>
</br>


