# Interior Mutability в Rust

## Что такое interior mutability

**Interior mutability** (внутренняя изменяемость) — паттерн, при котором значение **мутируется** через **shared-ссылку** `&T`, а **не** через `&mut T`.

**Обычное правило:** `&T` → **только чтение**, `&mut T` → **изменение**.
**Interior mutability:** `&T` → **можно изменять** (с **проверками**).

## Зачем это нужно

Иногда структура **семантически** **immutable** снаружи, но **внутри** **кэширует** данные:

```rust
struct Cache {
    data: RefCell<HashMap<String, String>>,
}

impl Cache {
    fn get(&self, key: &str) -> Option<String> {   // ← &self, не &mut
        // но внутри меняем HashMap
    }
}
```

**Без interior mutability** пришлось бы **`&mut self`** → **неудобно** для **кэша**.

## Типы с interior mutability

### 1. `Cell<T>` — для `Copy`

```rust
use std::cell::Cell;

let c = Cell::new(42);
c.set(100);           // ← &self
println!("{}", c.get());   // 100
```

- **Только** для **`Copy`** или **`Default`**.
- **`get`/`set`/`replace`/`take`**.
- **Не даёт** ссылок **наружу**.
- **Без runtime-проверок** (нет aliasing).

### 2. `RefCell<T>` — для **любых** типов

```rust
use std::cell::RefCell;

let c = RefCell::new(vec![1, 2, 3]);
c.borrow_mut().push(4);   // ← &self
println!("{:?}", c.borrow());   // [1, 2, 3, 4]
```

- **`borrow()`** → `Ref<T>` (shared).
- **`borrow_mut()`** → `RefMut<T>` (mutable).
- **Runtime-проверка** XOR.
- **Паника** при нарушении.

### 3. `Mutex<T>` / `RwLock<T>` — многопоточные

```rust
use std::sync::Mutex;

let m = Mutex::new(0);
*m.lock().unwrap() += 1;   // ← &self
```

- **`Sync`** — многопоточность.
- **Блокировка** потока.

### 4. `OnceCell<T>` / `LazyLock<T>` — однократная инициализация

```rust
use std::cell::OnceCell;

let c = OnceCell::new();
c.get_or_init(|| "hello");
```

- **Один раз** инициализируется.
- **Только чтение** после.

### 5. `Atomic*` — lock-free

```rust
use std::sync::atomic::{AtomicUsize, Ordering};

let a = AtomicUsize::new(0);
a.fetch_add(1, Ordering::SeqCst);   // ← &self
```

- **`Sync`**.
- **Lock-free**.
- **Только** для **примитивов**.

## Разбор примера

```rust
use std::cell::RefCell;
use std::collections::HashMap;

struct MyCache {
    d: RefCell<HashMap<String, String>>,
}

impl MyCache {
    fn get_or_insert(&self, k: &str, v: String) -> String {
        let mut m = self.d.borrow_mut();   // ← &self → &mut
        m.entry(k.to_string())
            .or_insert(v)
            .clone()
    }
}

fn main() {
    let my_cache = MyCache { d: RefCell::new(HashMap::new()) };

    let v = my_cache.get_or_insert("abc", "def".to_string());
    println!("{}", v);   // def
}
```

### Что происходит

1. **`MyCache`** **immutable** снаружи.
2. **`get_or_insert(&self, ...)`** — **`&self`**.
3. **`self.d.borrow_mut()`** — **внутренняя** изменяемость.
4. **`entry(...).or_insert(...)`** — **изменяет** HashMap.
5. **`.clone()`** — возвращает **копию** значения.

### Почему `&self`, а не `&mut self`

**Кэш** **семантически** **immutable** — снаружи **не важно**, что внутри **изменилось**. `&self` **удобнее**.

## `RefCell` — runtime-проверка XOR

```rust
use std::cell::RefCell;

let c = RefCell::new(42);

let b1 = c.borrow();       // shared
let b2 = c.borrow();       // ✅ ещё shared

// let b3 = c.borrow_mut();   // ❌ panic!

drop(b1);
drop(b2);

let b4 = c.borrow_mut();   // ✅ теперь можно
```

**Паника** при нарушении:

```
thread 'main' panicked at 'already borrowed: BorrowMutError'
```

## `UnsafeCell` — основа всего

**Все** типы interior mutability построены на **`UnsafeCell<T>`**:

```rust
pub struct UnsafeCell<T: ?Sized> {
    value: T,
}

impl<T> UnsafeCell<T> {
    pub const fn get(&self) -> *mut T;   // ← &self → *mut T
}
```

- **`&UnsafeCell<T>`** → **`*mut T`**.
- **`UnsafeCell`** — **единственный** способ получить **`&mut T`** из **`&self`**.
- **Прямое** использование — **`unsafe`**.
- **Оборачивается** в **безопасные** типы (`Cell`, `RefCell`, `Mutex`).

## Сводная таблица типов

| Тип | Для чего | Потокобезопасность | Проверка |
|---|---|---|---|
| **`Cell<T>`** | `Copy`-типы | ❌ `!Sync` | Compile-time (нет ссылок) |
| **`RefCell<T>`** | Любые типы | ❌ `!Sync` | **Runtime** (panic) |
| **`Mutex<T>`** | Любые типы | ✅ `Sync` | **Блокировка** |
| **`RwLock<T>`** | Любые типы | ✅ `Sync` | **Блокировка** |
| **`OnceCell<T>`** | Однократно | ❌ `!Sync` | Однократность |
| **`OnceLock<T>`** | Однократно | ✅ `Sync` | Однократность |
| **`Atomic*`** | Примитивы | ✅ `Sync` | Атомарность |

## Сводная таблица методов

| Тип | Метод | Что делает |
|---|---|---|
| **`Cell`** | `get()` | Копирует |
| **`Cell`** | `set(v)` | Заменяет |
| **`Cell`** | `replace(v)` | Заменяет + возвращает старое |
| **`Cell`** | `take()` | Забирает + `Default` |
| **`RefCell`** | `borrow()` | `Ref<T>` (shared) |
| **`RefCell`** | `borrow_mut()` | `RefMut<T>` (mutable) |
| **`RefCell`** | `try_borrow()` | `Result` вместо panic |
| **`Mutex`** | `lock()` | `MutexGuard<T>` |
| **`RwLock`** | `read()` / `write()` | `RwLockReadGuard` / `RwLockWriteGuard` |

## Где нужно

### 1. **Кэш** / **memoization**

```rust
struct Memo<T> {
    cache: RefCell<HashMap<u32, T>>,
}

impl<T: Clone> Memo<T> {
    fn get_or_compute<F: FnOnce(u32) -> T>(&self, n: u32, f: F) -> T {
        let mut cache = self.cache.borrow_mut();
        cache.entry(n).or_insert_with(|| f(n)).clone()
    }
}
```

### 2. **Ленивая** инициализация

```rust
struct Lazy {
    value: OnceCell<String>,
}

impl Lazy {
    fn get(&self) -> &String {
        self.value.get_or_init(|| "expensive".to_string())
    }
}
```

### 3. **Графы** / **деревья**

```rust
struct Node {
    children: RefCell<Vec<Rc<Node>>>,
}
```

### 4. **Счётчики** в `&self`

```rust
struct Counter {
    count: Cell<u32>,
}

impl Counter {
    fn increment(&self) {
        self.count.set(self.count.get() + 1);
    }
}
```

## Сводная таблица

| Аспект | Описание |
|---|---|
| **Interior mutability** | Изменение через `&T` |
| **Основа** | `UnsafeCell` |
| **Однопоточные** | `Cell`, `RefCell` |
| **Многопоточные** | `Mutex`, `RwLock`, `Atomic*` |
| **Однократные** | `OnceCell`, `OnceLock`, `LazyLock` |
| **Проверки** | Compile-time / runtime / блокировка |

## Итог

- **Interior mutability** — **изменение** через **`&T`**.
- **Основа** — **`UnsafeCell<T>`** (единственный примитив).
- **Типы:**
  - **`Cell<T>`** — `Copy`, без ссылок;
  - **`RefCell<T>`** — любые типы, runtime XOR;
  - **`Mutex<T>`** / **`RwLock<T>`** — многопоточные;
  - **`OnceCell<T>`** / **`LazyLock<T>`** — однократные;
  - **`Atomic*`** — lock-free.
- **Применения:** кэш, ленивая инициализация, графы, счётчики.
- **`RefCell`** — **паника** при нарушении XOR.
- **В вашем примере:** `MyCache` — **кэш** через `RefCell<HashMap>`, `&self`.
- **Правило:** interior mutability — когда **`&self`** нужен, но **внутри** нужно **изменять**.
