use std::{io::Write, os::raw::c_void, ptr::null_mut, time::Duration};

use msfs_sys::{SimConnect_AddToDataDefinition, SimConnect_CallDispatch, SimConnect_ClearDataDefinition, SimConnect_Close, SimConnect_GetNextDispatch, SimConnect_Open, SimConnect_RequestDataOnSimObject, SimConnect_SubscribeToSystemEvent, HANDLE, HRESULT, SIMCONNECT_DATATYPE, SIMCONNECT_DATATYPE_FLOAT64, SIMCONNECT_DATA_REQUEST_FLAG_CHANGED, SIMCONNECT_DATA_REQUEST_FLAG_DEFAULT, SIMCONNECT_OBJECT_ID_USER, SIMCONNECT_PERIOD_ONCE, SIMCONNECT_PERIOD_SIM_FRAME, SIMCONNECT_RECV, SIMCONNECT_RECV_EXCEPTION, SIMCONNECT_RECV_ID_EXCEPTION, SIMCONNECT_RECV_ID_SIMOBJECT_DATA, SIMCONNECT_RECV_SIMOBJECT_DATA, SIMCONNECT_UNUSED};

fn main() {
    println!("Hello, world!");
    let sim = SimConnect::new().expect("Can't connect");
    reset_leds();
    for num in 1..19 {
        let leds = [(num, 0), (num+1, 7)];
        send_cmd(&leds);
        std::thread::sleep(std::time::Duration::from_millis(100));
    }
    send_cmd(&[(19,0)]);
    let def_id = 1;
    sim.create_data_definition(def_id).expect("Cannot create_data_definition");
    let req_id = 0;
    sim.request_on_changed(def_id, req_id).expect("Cannot request data");
    loop {
        match sim.process_event(req_id) {
            Err(e) => {
                eprintln!("process_event: error {e:#x}");
                std::thread::sleep(Duration::from_millis(200));
            }
            Ok(()) => {
                std::thread::sleep(Duration::from_millis(200));
            }
        }
    }
}

extern "C" fn dispatch(data_ptr: *mut SIMCONNECT_RECV, len: u32, ctx: *mut c_void) {
    eprintln!("dispatch({data_ptr:p}, {len}, {ctx:p})");
    if data_ptr.is_null() { return; }
    let req_id = ctx as u32;
    unsafe {
        let rcv_id = (*data_ptr).dwID as i32;
        match rcv_id {
            SIMCONNECT_RECV_ID_SIMOBJECT_DATA => {
                let data_ptr = data_ptr as *const SIMCONNECT_RECV_SIMOBJECT_DATA;
                if (*data_ptr).dwRequestID == req_id {
                    let data = &(*data_ptr).dwData as *const _ as *const Data;
                    println!("data_ptr: {data:p}");
                    let data = *data;
                    println!("{data:?}");
                    process_data(&data);
                }
            }
            SIMCONNECT_RECV_ID_EXCEPTION => {
                let data_ptr = data_ptr as *const SIMCONNECT_RECV_EXCEPTION;
                println!("Ex: {:#x}, Sid: {:#x}, Idx: {:#x}", (*data_ptr).dwException, (*data_ptr).dwSendID, (*data_ptr).dwIndex);
            }
            _ => {
                println!("{rcv_id}");
            }
        }
    }
}

/*
define LED_GEAR_NOSE         1
define LED_GEAR_LEFT         2
define LED_GEAR_RIGHT        3
define LED_GEAR_WARNING      19
define LED_RWR_SEARCH        14
define LED_RWR_A_POWER       15
define LED_RWR_LOW_ALT_RED   16
define LED_RWR_LOW_ALT_GREEN 17
define LED_RWR_SYSTEM_POWER  18

define LED_USER_LEFT_1 12
define LED_USER_LEFT_2 10
define LED_USER_LEFT_3 8
define LED_USER_LEFT_4 6
define LED_USER_LEFT_5 4

define LED_USER_RIGHT_1 13
define LED_USER_RIGHT_2 11
define LED_USER_RIGHT_3 9
define LED_USER_RIGHT_4 7
define LED_USER_RIGHT_5 5
 */

const OFF: u8 = 0;
const GREEN: u8 = 1;
const RED: u8 = 2;
const WHITE: u8 = 7;

const LED_GEAR_NOSE: u8 = 1;
const LED_GEAR_LEFT: u8 = 2;
const LED_GEAR_RIGHT: u8 = 3;
const LED_RWR_SEARCH: u8 = 14;
const LED_RWR_LOW_ALT_RED: u8 =   16;
const LED_RWR_LOW_ALT_GREEN: u8 = 17;

const LED_RIGHT_1: u8 = 13;
const LED_RIGHT_2: u8 = 11;
const LED_RIGHT_3: u8 = 9;
const LED_RIGHT_4: u8 = 7;
const LED_RIGHT_5: u8 = 5;

fn process_data(data: &Data) {
    let mut leds = Vec::with_capacity(19);
    let gears = [
        (data.gear_center_pos, LED_GEAR_NOSE), 
        (data.gear_left_pos, LED_GEAR_LEFT),
        (data.gear_right_pos, LED_GEAR_RIGHT),
    ];
    for (gear_pos, led) in gears.iter().copied() {
        if gear_pos < 0.1 { // retracted
            leds.push((led, OFF));
        } else if gear_pos < 0.9 { //moving
            leds.push((led, RED));
        } else if gear_pos < 1.1 { // extended
            leds.push((led, GREEN));
        } else {
            leds.push((led, WHITE));
        }
    }
    let flaps = (data.left_flaps + data.right_flaps) / 2.0;
    let flap_leds = [LED_RIGHT_5, LED_RIGHT_4, LED_RIGHT_3, LED_RIGHT_2, LED_RIGHT_1];
    for (idx, led) in flap_leds.into_iter().enumerate() {
        let bound = (idx + 1) as f64 / 6.0;
        if flaps > bound {
            leds.push((led, GREEN));
        } else {
            leds.push((led, OFF));
        }
    }
    if data.landing_light != 0 {
        leds.push((LED_RWR_SEARCH, GREEN));
    } else {
        leds.push((LED_RWR_SEARCH, OFF));
    }
    if data.low_height != 0 {
        leds.push((LED_RWR_LOW_ALT_RED, RED));
        leds.push((LED_RWR_LOW_ALT_GREEN, OFF));
    } else {
        leds.push((LED_RWR_LOW_ALT_RED, OFF));
        leds.push((LED_RWR_LOW_ALT_GREEN, OFF));
    };
    send_cmd(&leds);
}

fn reset_leds() {
    let reset: Vec<(u8, u8)> = (1..20).map(|idx| (idx, 0)).collect();
    send_cmd(&reset);
}

fn send_cmd(led_states: &[(u8, u8)]) {
    let Ok(mut sock) = std::net::TcpStream::connect("127.0.0.1:2323") else {
        eprintln!("Cannot connect to 127.0.0.1:2323");
        return;
    };
    let buf_len = 3 + 3 * led_states.len();
    let mut cmd = Vec::with_capacity(buf_len);
    cmd.push((buf_len & 0xff) as u8);
    cmd.push((buf_len / 256) as u8);
    cmd.push(b'u');
    for (num, state) in led_states.iter().copied() {
        cmd.push(b'0' + num / 10);
        cmd.push(b'0' + num % 10);
        cmd.push(b'0' + state);
    }
    if let Err(e) = sock.write_all(&cmd) {
        eprintln!("Can't write to socket: {e:?}");
    };
}


struct SimConnect {
    handle: HANDLE,
}

impl Drop for SimConnect {
    fn drop(&mut self) {
        unsafe {
            SimConnect_Close(self.handle);
        }
    }
}

macro_rules! sdk {
    ( $e:expr ) => {
        {
            let result = $e;
            if result != 0 {
                eprintln!("Error {result:#x} in {}", stringify!($e));
                return Err(result);
            };
        }
    };
}

#[derive(Debug, Clone, Copy)]
#[repr(C)]
struct Data {
    gear_center_pos: f64,
    gear_left_pos: f64,
    gear_right_pos: f64,
    left_flaps: f64,
    right_flaps: f64,
    landing_light: i64,
    low_height: i64,
}

impl SimConnect {
    pub fn new() -> Result<SimConnect, HRESULT> {
        let mut handle = null_mut();
        let name = b"ViperLed\0";
        unsafe {
            sdk!(SimConnect_Open(&mut handle, name.as_ptr() as *const _, null_mut(), 0, null_mut(), 0));
        }
        Ok(SimConnect{handle})
    }

    fn add_to_data_definition(&self, def_id: u32, sim_var: &str, units_name: &str, datum_type: SIMCONNECT_DATATYPE) -> Result<(), HRESULT> {
        let c_sim_var = std::ffi::CString::new(sim_var.to_string()).expect("sim_var");
        let c_units_name = std::ffi::CString::new(units_name.to_string()).expect("datum_type");
        unsafe {
            sdk!(SimConnect_AddToDataDefinition(self.handle, def_id, c_sim_var.as_ptr(), c_units_name.as_ptr(), datum_type, 0.1, SIMCONNECT_UNUSED));
        }
        Ok(())
    }

    fn add_to_data_definition_f64(&self, def_id: u32, sim_var: &str, units_name: &str) -> Result<(), HRESULT> {
        self.add_to_data_definition(def_id, sim_var, units_name, SIMCONNECT_DATATYPE_FLOAT64)
    }

    fn add_to_data_definition_i64(&self, def_id: u32, sim_var: &str, units_name: &str) -> Result<(), HRESULT> {
        self.add_to_data_definition(def_id, sim_var, units_name, SIMCONNECT_DATATYPE_FLOAT64)
    }

    pub fn create_data_definition(&self, def_id: u32) -> Result<(), HRESULT> {
        // unsafe {
        //     SimConnect_ClearDataDefinition(self.handle, def_id);
        // }
        self.add_to_data_definition_f64(def_id, "GEAR CENTER POSITION", "percent over 100")?;
        self.add_to_data_definition_f64(def_id, "GEAR LEFT POSITION", "percent over 100")?;
        self.add_to_data_definition_f64(def_id, "GEAR RIGHT POSITION", "percent over 100")?;
        self.add_to_data_definition_f64(def_id, "TRAILING EDGE FLAPS LEFT PERCENT", "percent over 100")?;
        self.add_to_data_definition_f64(def_id, "TRAILING EDGE FLAPS RIGHT PERCENT", "percent over 100")?;
        self.add_to_data_definition_i64(def_id, "LIGHT LANDING", "bool")?;
        self.add_to_data_definition_i64(def_id, "WARNING LOW HEIGHT", "bool")?;
       Ok(())
    }

    pub fn request_on_changed(&self, def_id: u32, req_id: u32) -> Result<(), HRESULT> {
        unsafe {
            sdk!(SimConnect_RequestDataOnSimObject(self.handle, req_id, def_id,
                SIMCONNECT_OBJECT_ID_USER,
                SIMCONNECT_PERIOD_SIM_FRAME,
                SIMCONNECT_DATA_REQUEST_FLAG_CHANGED,
                0, 10, 0));

            //sdk!(SimConnect_SubscribeToSystemEvent(self.handle, 0, b"SimStart\0".as_ptr() as *const i8));

        }
        Ok(())
    }

    pub fn request_once(&self, def_id: u32, req_id: u32) -> Result<(), HRESULT> {
        unsafe {
            sdk!(SimConnect_RequestDataOnSimObject(self.handle, req_id, def_id,
                SIMCONNECT_OBJECT_ID_USER,
                SIMCONNECT_PERIOD_ONCE,
                SIMCONNECT_DATA_REQUEST_FLAG_DEFAULT,
                0, 10, 0));

            // sdk!(SimConnect_SubscribeToSystemEvent(self.handle, 0, b"SimStart\0".as_ptr() as *const i8));

        }
        Ok(())
    }

    pub fn process_event(&self, req_id: u32) -> Result<(), HRESULT> {
        unsafe {
            sdk!(SimConnect_CallDispatch(self.handle, Some(dispatch), req_id as *mut c_void));
        }
        Ok(())
    }
}


