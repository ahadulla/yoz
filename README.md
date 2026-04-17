# yoz

Terminal matn muharriri, Rust tilida yozilgan.

`nano` kabi sodda va qulay — modal rejimlar yo'q, darhol yozishni boshlang.

## O'rnatish

```bash
cargo install --path .
```

## Ishlatish

```bash
yoz fayl.txt       # Faylni ochish (yo'q bo'lsa yaratiladi)
yoz                # Bo'sh buffer
```

## Imkoniyatlar

### Navigatsiya
| Shortcut | Vazifasi |
|---|---|
| Arrow keys | Kursor harakati |
| Home / End | Qator boshi / oxiri |
| Ctrl+Home / End | Fayl boshi / oxiri |
| Ctrl+Left / Right | So'z boshi / oxiri |
| Ctrl+Up / Down | Scroll (kursor qimirlamaydi) |
| PageUp / PageDown | Sahifama-sahifa |
| Mouse scroll | Tezlikka moslashuvchan scroll |
| Mouse click | Kursorni bosgan joyga qo'yish |

### Tanlash (Selection)
| Shortcut | Vazifasi |
|---|---|
| Shift+Arrow | Belgilab borish |
| Shift+Home / End | Qator boshi/oxirigacha |
| Ctrl+Shift+Left / Right | So'zlab tanlash |
| Ctrl+A | Hammasini tanlash |
| Ikki marta bosish | So'zni tanlash |
| Mouse drag | Mouse bilan tanlash |
| Esc | Tanlashni bekor qilish |

### Tahrirlash
| Shortcut | Vazifasi |
|---|---|
| Ctrl+C | Nusxa olish (Copy) |
| Ctrl+X | Kesib olish (Cut) |
| Ctrl+V | Qo'yish (Paste) |
| Ctrl+Z | Ortga qaytarish (Undo) |
| Ctrl+Y | Qayta bajarish (Redo) |
| Ctrl+D | Qatorni duplikat qilish |
| Ctrl+K | Qator oxirigacha o'chirish |
| Ctrl+Backspace | So'zni o'chirish (chapga) |
| Ctrl+Delete | So'zni o'chirish (o'ngga) |
| Tab | 4 ta probel |

### Qidirish
| Shortcut | Vazifasi |
|---|---|
| Ctrl+F | Qidirish |
| Ctrl+H | Almashtirish |
| Ctrl+N | Keyingi natija |
| Ctrl+P | Oldingi natija |

### Boshqa
| Shortcut | Vazifasi |
|---|---|
| Ctrl+S | Saqlash |
| Ctrl+E | Encoding tanlash |
| Ctrl+L | Qator raqamlari yoq/o'ch |
| Ctrl+Q | Chiqish |
| F1 | Yordam |

## Encoding qo'llab-quvvatlash

Fayl ochilganda encoding avtomatik aniqlanadi (BOM + statistik tahlil). Qo'lda o'zgartirish uchun `Ctrl+E` bosing.

Qo'llab-quvvatlanadigan encodinglar:
- UTF-8
- UTF-8 BOM
- UTF-16 LE / BE
- Windows-1251 (kirill)
- Windows-1252 (g'arbiy Yevropa)
- CP866 (DOS kirill)

## Scrollbar

- O'ng tomonda — ingichka chiziq, mouse ustiga borganida kapsula shakliga o'tadi
- Mouse bilan tortib yurish (drag) mumkin
- Scroll tezligi tez aylantirganda oshib boradi

## Ekran tashkil etilishi

```
+------------------------------------------+
| fayl.txt     UTF-8  12:5  100 qator      |  <- Status bar
| 1  fn main() {                         |  |
| 2      println!("salom");              |  |  <- Scrollbar
| 3  }                                   |  |
|                                           |
| Ctrl+S saqlash | Ctrl+Q chiqish           |  <- Xabar qatori
+------------------------------------------+
```

## Texnologiyalar

- **Rust** — xavfsiz va tez
- **crossterm** — cross-platform terminal boshqaruvi
- **encoding_rs** — Mozilla tomonidan encoding kutubxonasi
- **chardetng** — encoding avtomatik aniqlash
- **arboard** — tizim clipboard

## Litsenziya

MIT
