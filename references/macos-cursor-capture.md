# macOS cursor capture — the proven technique (reference)

How real software KVMs make the local cursor **leave** one screen and appear on
the other (extend, not mirror) on macOS. Reverse-engineered from
**Deskflow's `OSXScreen.mm`** (the maintained Barrier/Synergy successor).

> Deskflow is GPLv2. We do **not** copy its code. This file documents the
> *technique* and the *public/known Apple APIs* it uses, so we can implement it
> independently. If stuck, read the original for reference:
> https://github.com/deskflow/deskflow/blob/master/src/lib/platform/OSXScreen.mm
> Key functions there: `enter()`, `leave()`, `onMouseMove()`, `hideCursor()`,
> `showCursor()`, `warpCursor()`, `setZeroSuppressionInterval()`.

## Why our earlier tries failed
1. **`CGAssociateMouseAndMouseCursorPosition(false)` is foreground-only** — a
   background daemon can't freeze the cursor with it.
2. **Hiding the cursor from a background app needs a private CGS property**
   (`SetsCursorInBackground`). Without it `CGDisplayHideCursor` silently no-ops.
3. We warped to the **edge/anchor** instead of the **center**, so the cursor hit
   screen edges where deltas go to 0.
4. We didn't filter the **warp artifact** (a warp produces a huge bogus delta).
5. We didn't zero the **local events suppression interval** (the ~250 ms deadzone
   after a warp).

## The algorithm (server / "primary" screen)

Uses **absolute-position deltas** (what rdev already gives us) — NOT raw event
delta fields. The trick is to keep re-centering the cursor.

**On hand-off to the client (leave / becomes "active"):**
- `hide_local_cursor()`  (see below)
- `warp_to(center)`  and set `last = center`
- (once at startup) `CGSetLocalEventsSuppressionInterval(0.0)`

**On every mouse move while active:**
1. `dx = x - last_x`, `dy = y - last_y`   (x,y = current absolute position)
2. `warp_to(center)`; `last = center`      (keeps cursor off the edges)
3. **Bogus filter:** if `|dx| > center_x - 10` or `|dy| > center_y - 10`, drop it
   (it's a warp artifact ≈ center-to-edge distance).
4. Otherwise forward `(dx, dy)`. (Deskflow accumulates the fractional part for
   smoothness; integer deltas are fine to start.)

**On return to local (enter / becomes "inactive"):**
- `show_local_cursor()`
- `warp_to(center)`

`center = (screen_width/2, screen_height/2)` in points (matches rdev's coord
space; we already auto-detect the screen size).

## The APIs (FFI signatures)

CoreGraphics.framework:
```
fn CGMainDisplayID() -> u32;
fn CGWarpMouseCursorPosition(p: CGPoint) -> i32;          // moves cursor, no event
fn CGDisplayHideCursor(display: u32) -> i32;
fn CGDisplayShowCursor(display: u32) -> i32;
fn CGAssociateMouseAndMouseCursorPosition(connected: bool) -> i32; // call true after hide/show (visibility bug fix)
fn CGSetLocalEventsSuppressionInterval(seconds: f64) -> i32;       // deprecated but works; kills warp deadzone
// private CGS (undocumented, but in CoreGraphics; fine for non-App-Store):
fn _CGSDefaultConnection() -> i32;
fn CGSSetConnectionProperty(cid: i32, target: i32, key: CFStringRef, value: CFTypeRef) -> i32;
```
CoreFoundation.framework:
```
fn CFStringCreateWithCString(alloc, cstr: *const c_char, encoding: u32) -> CFStringRef; // encoding 0 = MacRoman
fn CFRelease(cf);
static kCFBooleanTrue: CFTypeRef;
```
`CGPoint { x: f64, y: f64 }` (repr C).

### hide_local_cursor()
```
key = CFStringCreateWithCString(null, "SetsCursorInBackground\0", 0);
CGSSetConnectionProperty(_CGSDefaultConnection(), _CGSDefaultConnection(), key, kCFBooleanTrue);
CFRelease(key);
CGDisplayHideCursor(CGMainDisplayID());
CGAssociateMouseAndMouseCursorPosition(true);   // "fixes mouse randomly not hiding"
```
### show_local_cursor()
Same, but `CGDisplayShowCursor` instead of hide.

## Notes / gotchas
- `CGWarpMouseCursorPosition` does not generate events (no feedback loop).
- macOS Tahoe: warping can trigger hot-corner watchers; acceptable for now.
- Consume (return None) all events while active so local apps don't react.
- Windows: rdev's grab already suppresses locally (returning None works), so this
  whole dance is macOS-only. Keyboard stays on rdev on both platforms.
