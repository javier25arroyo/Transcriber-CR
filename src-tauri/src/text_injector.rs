//! Inyección del texto en la ventana enfocada: guardar el portapapeles, poner
//! la transcripción, mandar Ctrl+V y devolver el portapapeles como estaba.
//!
//! El portapapeles es del usuario, no nuestro: la parte que importa acá es
//! devolverlo intacto. Por eso el algoritmo está detrás de un trait y se testea
//! sin tocar el portapapeles real; lo que no se puede testear (el Ctrl+V de
//! verdad) es una función chica y aparte. Ver ADR 0003.

/// El portapapeles del sistema, detrás de un trait para poder testear el
/// algoritmo sin tocar el portapapeles de quien corre los tests.
pub trait Clipboard {
    fn get(&mut self) -> Option<String>;
    fn set(&mut self, s: &str);
}

/// Pone `text` en el portapapeles, pega, y restaura lo que había.
///
/// `paste` se recibe como parámetro para que esta función siga siendo pura y
/// determinista. En producción es [`paste_ctrl_v`], que además espera a que la
/// app destino procese el Ctrl+V: si se restaurara el portapapeles antes de
/// eso, la app pegaría el contenido viejo del usuario.
pub fn inject<C: Clipboard>(clipboard: &mut C, paste: impl FnOnce(), text: &str) {
    if text.is_empty() {
        return;
    }

    // `None` (portapapeles vacío) se restaura como cadena vacía: la
    // transcripción es texto del usuario y no tiene por qué quedar ahí después.
    let original = clipboard.get().unwrap_or_default();

    clipboard.set(text);
    paste();
    clipboard.set(&original);
}

#[cfg(feature = "app")]
mod system {
    use super::Clipboard;
    use std::time::Duration;

    /// Cuánto esperar entre el Ctrl+V y la restauración del portapapeles. La
    /// app destino lee el portapapeles de forma asíncrona; restaurar demasiado
    /// rápido hace que pegue el contenido anterior.
    const PASTE_SETTLE: Duration = Duration::from_millis(120);

    pub struct SystemClipboard(arboard::Clipboard);

    impl SystemClipboard {
        pub fn new() -> Result<Self, arboard::Error> {
            arboard::Clipboard::new().map(Self)
        }
    }

    impl Clipboard for SystemClipboard {
        fn get(&mut self) -> Option<String> {
            self.0.get_text().ok()
        }

        fn set(&mut self, s: &str) {
            let _ = self.0.set_text(s.to_string());
        }
    }

    /// Manda Ctrl+V a la ventana enfocada y espera a que la app lo procese.
    pub fn paste_ctrl_v() {
        use enigo::{
            Direction::{Click, Press, Release},
            Enigo, Key, Keyboard, Settings,
        };

        let Ok(mut enigo) = Enigo::new(&Settings::default()) else {
            return;
        };
        let _ = enigo.key(Key::Control, Press);
        let _ = enigo.key(Key::Unicode('v'), Click);
        let _ = enigo.key(Key::Control, Release);
        std::thread::sleep(PASTE_SETTLE);
    }
}

#[cfg(feature = "app")]
pub use system::{paste_ctrl_v, SystemClipboard};

#[cfg(test)]
mod tests {
    use super::*;
    use std::cell::RefCell;
    use std::rc::Rc;

    /// Estado compartido del portapapeles de mentira: lo que contiene ahora y
    /// todo lo que se le escribió, en orden. Va detrás de un `Rc<RefCell<_>>`
    /// para que el test pueda mirarlo *desde adentro* del pegado, que es donde
    /// se juega el orden de las operaciones.
    #[derive(Default)]
    struct ClipState {
        value: Option<String>,
        writes: Vec<String>,
    }

    struct MockClip(Rc<RefCell<ClipState>>);

    impl MockClip {
        fn with(value: Option<&str>) -> (Self, Rc<RefCell<ClipState>>) {
            let state = Rc::new(RefCell::new(ClipState {
                value: value.map(str::to_string),
                writes: Vec::new(),
            }));
            (MockClip(Rc::clone(&state)), state)
        }
    }

    impl Clipboard for MockClip {
        fn get(&mut self) -> Option<String> {
            self.0.borrow().value.clone()
        }

        fn set(&mut self, s: &str) {
            let mut state = self.0.borrow_mut();
            state.value = Some(s.into());
            state.writes.push(s.into());
        }
    }

    #[test]
    fn inject_sets_text_then_restores_original() {
        let (mut clip, state) = MockClip::with(Some("ORIGINAL"));
        let mut pasted = false;

        inject(&mut clip, || pasted = true, "hola mundo");

        assert!(pasted);
        assert_eq!(state.borrow().writes, vec!["hola mundo", "ORIGINAL"]);
        assert_eq!(state.borrow().value.as_deref(), Some("ORIGINAL"));
    }

    #[test]
    fn the_clipboard_holds_the_transcription_at_the_moment_of_the_paste() {
        // El orden es todo: pegar antes de poner el texto pegaría lo viejo, y
        // restaurar antes de pegar le daría a la app el contenido anterior del
        // usuario. Este test mira el portapapeles justo cuando se pega.
        let (mut clip, state) = MockClip::with(Some("ORIGINAL"));
        let seen_at_paste = RefCell::new(None);

        inject(
            &mut clip,
            || *seen_at_paste.borrow_mut() = state.borrow().value.clone(),
            "hola mundo",
        );

        assert_eq!(seen_at_paste.into_inner().as_deref(), Some("hola mundo"));
    }

    #[test]
    fn inject_leaves_an_empty_clipboard_empty() {
        // Sin contenido previo no hay nada que restaurar, pero dejar ahí la
        // transcripción tampoco: es texto dictado por el usuario y no tiene por
        // qué quedar en el portapapeles después.
        let (mut clip, state) = MockClip::with(None);

        inject(&mut clip, || {}, "hola mundo");

        assert_eq!(state.borrow().writes, vec!["hola mundo", ""]);
        assert_eq!(state.borrow().value.as_deref(), Some(""));
    }

    #[test]
    fn inject_does_nothing_with_empty_text() {
        // Un segmento vacío (silencio que igual llegó al motor) no puede pisar
        // el portapapeles ni mandar un Ctrl+V al vacío.
        let (mut clip, state) = MockClip::with(Some("ORIGINAL"));
        let mut pasted = false;

        inject(&mut clip, || pasted = true, "");

        assert!(!pasted);
        assert!(state.borrow().writes.is_empty());
        assert_eq!(state.borrow().value.as_deref(), Some("ORIGINAL"));
    }
}
