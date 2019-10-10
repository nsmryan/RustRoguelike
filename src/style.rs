use imgui::*;

// dark theme from codz01 (https://github.com/ocornut/imgui/issues/707)
pub fn set_style_dark(style: &mut Style) {
    style.frame_border_size = 1.0;
    style.frame_padding = [4.0,2.0];
    style.item_spacing = [8.0,2.0];
    style.window_border_size = 1.0;
    //style.tab_border_size = 1.0;
    style.window_rounding = 1.0;
    style.child_rounding = 1.0;
    style.frame_rounding = 1.0;
    style.scrollbar_rounding = 1.0;
    style.grab_rounding = 1.0;

    style.colors =
        [
        [1.00, 1.00, 1.00, 0.95], // ImGuiCol_Text 
        [0.50, 0.50, 0.50, 1.00], // ImGuiCol_TextDisabled 
        [0.13, 0.12, 0.12, 1.00], // ImGuiCol_WindowBg 
        [1.00, 1.00, 1.00, 0.00], // ImGuiCol_ChildBg 
        [0.05, 0.05, 0.05, 0.94], // ImGuiCol_PopupBg 
        [0.53, 0.53, 0.53, 0.46], // ImGuiCol_Border 
        [0.00, 0.00, 0.00, 0.00], // ImGuiCol_BorderShadow 
        [0.00, 0.00, 0.00, 0.85], // ImGuiCol_FrameBg 
        [0.22, 0.22, 0.22, 0.40], // ImGuiCol_FrameBgHovered 
        [0.16, 0.16, 0.16, 0.53], // ImGuiCol_FrameBgActive 
        [0.00, 0.00, 0.00, 1.00], // ImGuiCol_TitleBg 
        [0.00, 0.00, 0.00, 1.00], // ImGuiCol_TitleBgActive 
        [0.00, 0.00, 0.00, 0.51], // ImGuiCol_TitleBgCollapsed 
        [0.12, 0.12, 0.12, 1.00], // ImGuiCol_MenuBarBg 
        [0.02, 0.02, 0.02, 0.53], // ImGuiCol_ScrollbarBg 
        [0.31, 0.31, 0.31, 1.00], // ImGuiCol_ScrollbarGrab 
        [0.41, 0.41, 0.41, 1.00], // ImGuiCol_ScrollbarGrabHovered 
        [0.48, 0.48, 0.48, 1.00], // ImGuiCol_ScrollbarGrabActive 
        [0.79, 0.79, 0.79, 1.00], // ImGuiCol_CheckMark 
        [0.48, 0.47, 0.47, 0.91], // ImGuiCol_SliderGrab 
        [0.56, 0.55, 0.55, 0.62], // ImGuiCol_SliderGrabActive 
        [0.50, 0.50, 0.50, 0.63], // ImGuiCol_Button 
        [0.67, 0.67, 0.68, 0.63], // ImGuiCol_ButtonHovered 
        [0.26, 0.26, 0.26, 0.63], // ImGuiCol_ButtonActive 
        [0.54, 0.54, 0.54, 0.58], // ImGuiCol_Header 
        [0.64, 0.65, 0.65, 0.80], // ImGuiCol_HeaderHovered 
        [0.25, 0.25, 0.25, 0.80], // ImGuiCol_HeaderActive 
        [0.58, 0.58, 0.58, 0.50], // ImGuiCol_Separator 
        [0.81, 0.81, 0.81, 0.64], // ImGuiCol_SeparatorHovered 
        [0.81, 0.81, 0.81, 0.64], // ImGuiCol_SeparatorActive 
        [0.87, 0.87, 0.87, 0.53], // ImGuiCol_ResizeGrip 
        [0.87, 0.87, 0.87, 0.74], // ImGuiCol_ResizeGripHovered 
        [0.87, 0.87, 0.87, 0.74], // ImGuiCol_ResizeGripActive 
        [0.61, 0.61, 0.61, 1.00], // ImGuiCol_PlotLines 
        [0.68, 0.68, 0.68, 1.00], // ImGuiCol_PlotLinesHovered 
        [0.90, 0.77, 0.33, 1.00], // ImGuiCol_PlotHistogram 
        [0.87, 0.55, 0.08, 1.00], // ImGuiCol_PlotHistogramHovered 
        [0.47, 0.60, 0.76, 0.47], // ImGuiCol_TextSelectedBg 
        [0.58, 0.58, 0.58, 0.90], // ImGuiCol_DragDropTarget 
        [0.60, 0.60, 0.60, 1.00], // ImGuiCol_NavHighlight 
        [1.00, 1.00, 1.00, 0.70], // ImGuiCol_NavWindowingHighlight 
        [0.80, 0.80, 0.80, 0.20], // ImGuiCol_NavWindowingDimBg 
        [0.80, 0.80, 0.80, 0.35], // ImGuiCol_ModalWindowDimBg 
        [0.20, 0.20, 0.20, 0.35], // idk
        [0.20, 0.20, 0.20, 0.35], // idk
        [0.20, 0.20, 0.20, 0.35], // idk
        [0.20, 0.20, 0.20, 0.35], // idk
        [0.20, 0.20, 0.20, 0.35], // idk
        ];
}

// light green from @ebachard (https://github.com/ocornut/imgui/issues/707)
pub fn set_style_light(style: &mut Style) {
    style.window_rounding     = 2.0;
    style.scrollbar_rounding  = 3.0;
    style.grab_rounding       = 2.0;
    style.anti_aliased_lines  = true;
    style.anti_aliased_fill   = true;
    style.window_rounding     = 2.0;
    style.child_rounding      = 2.0;
    style.scrollbar_size      = 16.0;
    style.scrollbar_rounding  = 3.0;
    style.grab_rounding       = 2.0;
    style.item_spacing[0]      = 10.0;
    style.item_spacing[1]      = 4.0;
    style.indent_spacing      = 22.0;
    style.frame_padding[0]     = 6.0;
    style.frame_padding[1]     = 4.0;
    style.alpha               = 1.0;
    style.frame_rounding      = 3.0;

    style.colors =
        [
        [0.00, 0.00, 0.00, 1.00], // ImGuiCol_Text
        [0.60, 0.60, 0.60, 1.00], // ImGuiCol_TextDisabled
        [0.86, 0.86, 0.86, 1.00], // ImGuiCol_WindowBg
        [0.00, 0.00, 0.00, 0.00], // ImGuiCol_ChildBg
        [0.93, 0.93, 0.93, 0.98], // ImGuiCol_PopupBg
        [0.71, 0.71, 0.71, 0.08], // ImGuiCol_Border
        [0.00, 0.00, 0.00, 0.04], // ImGuiCol_BorderShadow
        [0.71, 0.71, 0.71, 0.55], // ImGuiCol_FrameBg
        [0.94, 0.94, 0.94, 0.55], // ImGuiCol_FrameBgHovered
        [0.71, 0.78, 0.69, 0.98], // ImGuiCol_FrameBgActive
        [0.85, 0.85, 0.85, 1.00], // ImGuiCol_TitleBg
        [0.78, 0.78, 0.78, 1.00], // ImGuiCol_TitleBgActive
        [0.82, 0.78, 0.78, 0.51], // ImGuiCol_TitleBgCollapsed
        [0.86, 0.86, 0.86, 1.00], // ImGuiCol_MenuBarBg
        [0.20, 0.25, 0.30, 0.61], // ImGuiCol_ScrollbarBg
        [0.90, 0.90, 0.90, 0.30], // ImGuiCol_ScrollbarGrab
        [0.92, 0.92, 0.92, 0.78], // ImGuiCol_ScrollbarGrabHovered
        [1.00, 1.00, 1.00, 1.00], // ImGuiCol_ScrollbarGrabActive
        [0.184, 0.407, 0.193, 1.00], // ImGuiCol_CheckMark
        [0.26, 0.59, 0.98, 0.78], // ImGuiCol_SliderGrab
        [0.26, 0.59, 0.98, 1.00], // ImGuiCol_SliderGrabActive
        [0.71, 0.78, 0.69, 0.40], // ImGuiCol_Button
        [0.725, 0.805, 0.702, 1.00], // ImGuiCol_ButtonHovered
        [0.793, 0.900, 0.836, 1.00], // ImGuiCol_ButtonActive
        [0.71, 0.78, 0.69, 0.31], // ImGuiCol_Header
        [0.71, 0.78, 0.69, 0.80], // ImGuiCol_HeaderHovered
        [0.71, 0.78, 0.69, 1.00], // ImGuiCol_HeaderActive
        [0.39, 0.39, 0.39, 1.00], // ImGuiCol_Separator
        [0.14, 0.44, 0.80, 0.78], // ImGuiCol_SeparatorHovered
        [0.14, 0.44, 0.80, 1.00], // ImGuiCol_SeparatorActive
        [1.00, 1.00, 1.00, 0.00], // ImGuiCol_ResizeGrip
        [0.26, 0.59, 0.98, 0.45], // ImGuiCol_ResizeGripHovered
        [0.26, 0.59, 0.98, 0.78], // ImGuiCol_ResizeGripActive
        [0.39, 0.39, 0.39, 1.00], // ImGuiCol_PlotLines
        [1.00, 0.43, 0.35, 1.00], // ImGuiCol_PlotLinesHovered
        [0.90, 0.70, 0.00, 1.00], // ImGuiCol_PlotHistogram
        [1.00, 0.60, 0.00, 1.00], // ImGuiCol_PlotHistogramHovered
        [0.26, 0.59, 0.98, 0.35], // ImGuiCol_TextSelectedBg
        [0.26, 0.59, 0.98, 0.95], // ImGuiCol_DragDropTarget
        [0.71, 0.78, 0.69, 0.80], // ImGuiCol_NavHighlight 
        [0.70, 0.70, 0.70, 0.70], // ImGuiCol_NavWindowingHighlight 
        [0.70, 0.70, 0.70, 0.30], // ImGuiCol_NavWindowingHighlight 
        [0.20, 0.20, 0.20, 0.35], // ImGuiCol_ModalWindowDarkening
        [0.20, 0.20, 0.20, 0.35], // idk
        [0.20, 0.20, 0.20, 0.35], // idk
        [0.20, 0.20, 0.20, 0.35], // idk
        [0.20, 0.20, 0.20, 0.35], // idk
        [0.20, 0.20, 0.20, 0.35], // idk
        ];
}

