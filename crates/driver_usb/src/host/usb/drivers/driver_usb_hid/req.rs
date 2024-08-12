



fn sent_key_event(){

}



fn main(){
    let report_desc_len = hid_desc.desc_len;
    assert_eq!(hid_desc.desc_ty, REPORT_DESC_TY);
    let mut left_shift = false;
    let mut right_shift = false;
    let mut last_mouse_pos = (0, 0);
    let mut last_buttons = [false, false, false];
    loop{
        busy_wait();


        let mut mouse_pos = last_mouse_pos;
        let mut mouse_dx = 0i32;
        let mut mouse_dy = 0i32;
        let mut scroll_y = 0i32;
        let mut buttons = last_buttons;

    }
}

