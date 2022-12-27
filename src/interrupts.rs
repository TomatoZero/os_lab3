use lazy_static::lazy_static;
use x86_64::structures::idt::{InterruptStackFrame, InterruptDescriptorTable};
use pic8259::ChainedPics;
use x86_64::instructions::port::Port;
use pc_keyboard::{layouts, DecodedKey, HandleControl, Keyboard, ScancodeSet1};
use spin::Mutex;

const PIC_1_OFFSET: u8 = 32;
const PIC_2_OFFSET: u8 = PIC_1_OFFSET + 8;
const TIMER_INTERRUPT: u8 = PIC_1_OFFSET;
const KEYBOARD_INTERRUPT: u8 = PIC_1_OFFSET + 1;

lazy_static! {
    static ref IDT: InterruptDescriptorTable = {
        let mut idt = InterruptDescriptorTable::new();
        idt.double_fault.set_handler_fn(double_fault_handler);
        idt[TIMER_INTERRUPT as usize].set_handler_fn(timer_interrupt_handler);
        idt[KEYBOARD_INTERRUPT as usize].set_handler_fn(keyboard_interrupt_handler);
        idt
    };
}

lazy_static! {
    static ref KEYBOARD: Mutex<Keyboard<layouts::Us104Key, ScancodeSet1>> = Mutex::new(
        Keyboard::new(
            layouts::Us104Key, 
            ScancodeSet1,
            HandleControl::Ignore
        )
    );
}

lazy_static! {
    static ref CUSTOM_HANDLERS: Mutex<CustomHandlers> = Mutex::new(
        {
            let mut ch = CustomHandlers{
                timer_interrupt_handler: || {},
                keyboard_interrupt_handler: |dk| {}
            };
            ch
        }
    );
}

static PICS: Mutex<ChainedPics> = spin::Mutex::new(
    unsafe { ChainedPics::new(PIC_1_OFFSET, PIC_2_OFFSET) }
);

pub fn init() {
    IDT.load();
    unsafe { PICS.lock().initialize() }
    x86_64::instructions::interrupts::enable();
}

pub fn set_keyboard_interrupt_handler(handler: fn(DecodedKey)) {
    CUSTOM_HANDLERS.lock().keyboard_interrupt_handler  = handler;
}

pub fn set_timer_interrupt_handler(handler: fn()) {
    CUSTOM_HANDLERS.lock().timer_interrupt_handler  = handler;
}

struct CustomHandlers {
    timer_interrupt_handler: fn(),
    keyboard_interrupt_handler: fn(DecodedKey),
}

extern "x86-interrupt" fn double_fault_handler(stack_frame: InterruptStackFrame, _error_code: u64) -> ! {
    panic!("DOUBLE FAULT\n{:#?}", stack_frame);
}

extern "x86-interrupt" fn timer_interrupt_handler(_stack_frame: InterruptStackFrame) {
    // delegate call to custom handlers function
    (CUSTOM_HANDLERS.lock().timer_interrupt_handler)();
    unsafe {
        PICS.lock().notify_end_of_interrupt(TIMER_INTERRUPT);
    }
}

extern "x86-interrupt" fn keyboard_interrupt_handler(_stack_frame: InterruptStackFrame) {
    let mut keyboard = KEYBOARD.lock();
    let mut port = Port::new(0x60);

    let scancode: u8 = unsafe { port.read() };
    if let Ok(Some(key_event)) = keyboard.add_byte(scancode) {
        if let Some(key) = keyboard.process_keyevent(key_event) {
            // delegate call to custom handlers function
            (CUSTOM_HANDLERS.lock().keyboard_interrupt_handler)(key);
        }
    }

    unsafe {
        PICS.lock().notify_end_of_interrupt(KEYBOARD_INTERRUPT);
    }
}
