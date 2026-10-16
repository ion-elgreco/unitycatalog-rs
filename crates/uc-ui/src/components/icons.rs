//! antd's icons (`@ant-design/icons`, MIT), as inline SVG: the glyphs of
//! `@ant-design/icons-svg`, under the names that upstream's UI uses.

use leptos::prelude::*;

/// An icon's SVG data: its view box and paths.
struct Glyph {
    view_box: &'static str,
    /// The paths' `fill-rule` is `evenodd`.
    even_odd: bool,
    paths: &'static [&'static str],
}

/// An icon as antd draws it: `1em` square, in the text's color.
fn icon(glyph: &'static Glyph) -> impl IntoView {
    view! {
        <span class="uc-icon" role="img">
            <svg
                viewBox=glyph.view_box
                fill-rule=glyph.even_odd.then_some("evenodd")
                width="1em"
                height="1em"
                fill="currentColor"
                aria-hidden="true"
                focusable="false"
            >
                {glyph.paths.iter().map(|d| view! { <path d=*d/> }).collect_view()}
            </svg>
        </span>
    }
}

const PRODUCT_OUTLINED: Glyph = Glyph {
    view_box: "64 64 896 896",
    even_odd: true,
    paths: &[
        "M464 144a16 16 0 0116 16v304a16 16 0 01-16 16H160a16 16 0 01-16-16V160a16 16 0 0116-16zm-52 68H212v200h200zm493.33 87.69a16 16 0 010 22.62L724.31 503.33a16 16 0 01-22.62 0L520.67 322.31a16 16 0 010-22.62l181.02-181.02a16 16 0 0122.62 0zm-84.85 11.3L713 203.53 605.52 311 713 418.48zM464 544a16 16 0 0116 16v304a16 16 0 01-16 16H160a16 16 0 01-16-16V560a16 16 0 0116-16zm-52 68H212v200h200zm452-68a16 16 0 0116 16v304a16 16 0 01-16 16H560a16 16 0 01-16-16V560a16 16 0 0116-16zm-52 68H612v200h200z",
    ],
};

/// A catalog.
#[component]
pub fn ProductOutlined() -> impl IntoView {
    icon(&PRODUCT_OUTLINED)
}

const DATABASE_OUTLINED: Glyph = Glyph {
    view_box: "64 64 896 896",
    even_odd: false,
    paths: &[
        "M832 64H192c-17.7 0-32 14.3-32 32v832c0 17.7 14.3 32 32 32h640c17.7 0 32-14.3 32-32V96c0-17.7-14.3-32-32-32zm-600 72h560v208H232V136zm560 480H232V408h560v208zm0 272H232V680h560v208zM304 240a40 40 0 1080 0 40 40 0 10-80 0zm0 272a40 40 0 1080 0 40 40 0 10-80 0zm0 272a40 40 0 1080 0 40 40 0 10-80 0z",
    ],
};

/// A schema.
#[component]
pub fn DatabaseOutlined() -> impl IntoView {
    icon(&DATABASE_OUTLINED)
}

const TABLE_OUTLINED: Glyph = Glyph {
    view_box: "64 64 896 896",
    even_odd: false,
    paths: &[
        "M928 160H96c-17.7 0-32 14.3-32 32v640c0 17.7 14.3 32 32 32h832c17.7 0 32-14.3 32-32V192c0-17.7-14.3-32-32-32zm-40 208H676V232h212v136zm0 224H676V432h212v160zM412 432h200v160H412V432zm200-64H412V232h200v136zm-476 64h212v160H136V432zm0-200h212v136H136V232zm0 424h212v136H136V656zm276 0h200v136H412V656zm476 136H676V656h212v136z",
    ],
};

/// A table.
#[component]
pub fn TableOutlined() -> impl IntoView {
    icon(&TABLE_OUTLINED)
}

const FOLDER_OUTLINED: Glyph = Glyph {
    view_box: "64 64 896 896",
    even_odd: false,
    paths: &[
        "M880 298.4H521L403.7 186.2a8.15 8.15 0 00-5.5-2.2H144c-17.7 0-32 14.3-32 32v592c0 17.7 14.3 32 32 32h736c17.7 0 32-14.3 32-32V330.4c0-17.7-14.3-32-32-32zM840 768H184V256h188.5l119.6 114.4H840V768z",
    ],
};

/// A volume.
#[component]
pub fn FolderOutlined() -> impl IntoView {
    icon(&FOLDER_OUTLINED)
}

const FUNCTION_OUTLINED: Glyph = Glyph {
    view_box: "64 64 896 896",
    even_odd: false,
    paths: &[
        "M841 370c3-3.3 2.7-8.3-.6-11.3a8.24 8.24 0 00-5.3-2.1h-72.6c-2.4 0-4.6 1-6.1 2.8L633.5 504.6a7.96 7.96 0 01-13.4-1.9l-63.5-141.3a7.9 7.9 0 00-7.3-4.7H380.7l.9-4.7 8-42.3c10.5-55.4 38-81.4 85.8-81.4 18.6 0 35.5 1.7 48.8 4.7l14.1-66.8c-22.6-4.7-35.2-6.1-54.9-6.1-103.3 0-156.4 44.3-175.9 147.3l-9.4 49.4h-97.6c-3.8 0-7.1 2.7-7.8 6.4L181.9 415a8.07 8.07 0 007.8 9.7H284l-89 429.9a8.07 8.07 0 007.8 9.7H269c3.8 0 7.1-2.7 7.8-6.4l89.7-433.1h135.8l68.2 139.1c1.4 2.9 1 6.4-1.2 8.8l-180.6 203c-2.9 3.3-2.6 8.4.7 11.3 1.5 1.3 3.4 2 5.3 2h72.7c2.4 0 4.6-1 6.1-2.8l123.7-146.7c2.8-3.4 7.9-3.8 11.3-1 .9.8 1.6 1.7 2.1 2.8L676.4 784c1.3 2.8 4.1 4.7 7.3 4.7h64.6a8.02 8.02 0 007.2-11.5l-95.2-198.9c-1.4-2.9-.9-6.4 1.3-8.8L841 370z",
    ],
};

/// A function.
#[component]
pub fn FunctionOutlined() -> impl IntoView {
    icon(&FUNCTION_OUTLINED)
}

const DEPLOYMENT_UNIT_OUTLINED: Glyph = Glyph {
    view_box: "64 64 896 896",
    even_odd: false,
    paths: &[
        "M888.3 693.2c-42.5-24.6-94.3-18-129.2 12.8l-53-30.7V523.6c0-15.7-8.4-30.3-22-38.1l-136-78.3v-67.1c44.2-15 76-56.8 76-106.1 0-61.9-50.1-112-112-112s-112 50.1-112 112c0 49.3 31.8 91.1 76 106.1v67.1l-136 78.3c-13.6 7.8-22 22.4-22 38.1v151.6l-53 30.7c-34.9-30.8-86.8-37.4-129.2-12.8-53.5 31-71.7 99.4-41 152.9 30.8 53.5 98.9 71.9 152.2 41 42.5-24.6 62.7-73 53.6-118.8l48.7-28.3 140.6 81c6.8 3.9 14.4 5.9 22 5.9s15.2-2 22-5.9L674.5 740l48.7 28.3c-9.1 45.7 11.2 94.2 53.6 118.8 53.3 30.9 121.5 12.6 152.2-41 30.8-53.6 12.6-122-40.7-152.9zm-673 138.4a47.6 47.6 0 01-65.2-17.6c-13.2-22.9-5.4-52.3 17.5-65.5a47.6 47.6 0 0165.2 17.6c13.2 22.9 5.4 52.3-17.5 65.5zM522 463.8zM464 234a48.01 48.01 0 0196 0 48.01 48.01 0 01-96 0zm170 446.2l-122 70.3-122-70.3V539.8l122-70.3 122 70.3v140.4zm239.9 133.9c-13.2 22.9-42.4 30.8-65.2 17.6-22.8-13.2-30.7-42.6-17.5-65.5s42.4-30.8 65.2-17.6c22.9 13.2 30.7 42.5 17.5 65.5z",
    ],
};

/// A registered model.
#[component]
pub fn DeploymentUnitOutlined() -> impl IntoView {
    icon(&DEPLOYMENT_UNIT_OUTLINED)
}

const EDIT_OUTLINED: Glyph = Glyph {
    view_box: "64 64 896 896",
    even_odd: false,
    paths: &[
        "M257.7 752c2 0 4-.2 6-.5L431.9 722c2-.4 3.9-1.3 5.3-2.8l423.9-423.9a9.96 9.96 0 000-14.1L694.9 114.9c-1.9-1.9-4.4-2.9-7.1-2.9s-5.2 1-7.1 2.9L256.8 538.8c-1.5 1.5-2.4 3.3-2.8 5.3l-29.5 168.2a33.5 33.5 0 009.4 29.8c6.6 6.4 14.9 9.9 23.8 9.9zm67.4-174.4L687.8 215l73.3 73.3-362.7 362.6-88.9 15.7 15.6-89zM880 836H144c-17.7 0-32 14.3-32 32v36c0 4.4 3.6 8 8 8h784c4.4 0 8-3.6 8-8v-36c0-17.7-14.3-32-32-32z",
    ],
};

/// Edit a description.
#[component]
pub fn EditOutlined() -> impl IntoView {
    icon(&EDIT_OUTLINED)
}

const DELETE_OUTLINED: Glyph = Glyph {
    view_box: "64 64 896 896",
    even_odd: false,
    paths: &[
        "M360 184h-8c4.4 0 8-3.6 8-8v8h304v-8c0 4.4 3.6 8 8 8h-8v72h72v-80c0-35.3-28.7-64-64-64H352c-35.3 0-64 28.7-64 64v80h72v-72zm504 72H160c-17.7 0-32 14.3-32 32v32c0 4.4 3.6 8 8 8h60.4l24.7 523c1.6 34.1 29.8 61 63.9 61h454c34.2 0 62.3-26.8 63.9-61l24.7-523H888c4.4 0 8-3.6 8-8v-32c0-17.7-14.3-32-32-32zM731.3 840H292.7l-24.2-512h487l-24.2 512z",
    ],
};

/// A delete in a menu.
#[component]
pub fn DeleteOutlined() -> impl IntoView {
    icon(&DELETE_OUTLINED)
}

const MORE_OUTLINED: Glyph = Glyph {
    view_box: "64 64 896 896",
    even_odd: false,
    paths: &[
        "M456 231a56 56 0 10112 0 56 56 0 10-112 0zm0 280a56 56 0 10112 0 56 56 0 10-112 0zm0 280a56 56 0 10112 0 56 56 0 10-112 0z",
    ],
};

/// The action menu's button.
#[component]
pub fn MoreOutlined() -> impl IntoView {
    icon(&MORE_OUTLINED)
}

const SEARCH_OUTLINED: Glyph = Glyph {
    view_box: "64 64 896 896",
    even_odd: false,
    paths: &[
        "M909.6 854.5L649.9 594.8C690.2 542.7 712 479 712 412c0-80.2-31.3-155.4-87.9-212.1-56.6-56.7-132-87.9-212.1-87.9s-155.5 31.3-212.1 87.9C143.2 256.5 112 331.8 112 412c0 80.1 31.3 155.5 87.9 212.1C256.5 680.8 331.8 712 412 712c67 0 130.6-21.8 182.7-62l259.7 259.6a8.2 8.2 0 0011.6 0l43.6-43.5a8.2 8.2 0 000-11.6zM570.4 570.4C528 612.7 471.8 636 412 636s-116-23.3-158.4-65.6C211.3 528 188 471.8 188 412s23.3-116.1 65.6-158.4C296 211.3 352.2 188 412 188s116.1 23.2 158.4 65.6S636 352.2 636 412s-23.3 116.1-65.6 158.4z",
    ],
};

/// The search box.
#[component]
pub fn SearchOutlined() -> impl IntoView {
    icon(&SEARCH_OUTLINED)
}

const CHECK_CIRCLE_OUTLINED: Glyph = Glyph {
    view_box: "64 64 896 896",
    even_odd: false,
    paths: &[
        "M699 353h-46.9c-10.2 0-19.9 4.9-25.9 13.3L469 584.3l-71.2-98.8c-6-8.3-15.6-13.3-25.9-13.3H325c-6.5 0-10.3 7.4-6.5 12.7l124.6 172.8a31.8 31.8 0 0051.7 0l210.6-292c3.9-5.3.1-12.7-6.4-12.7z",
        "M512 64C264.6 64 64 264.6 64 512s200.6 448 448 448 448-200.6 448-448S759.4 64 512 64zm0 820c-205.4 0-372-166.6-372-372s166.6-372 372-372 372 166.6 372 372-166.6 372-372 372z",
    ],
};

/// A ready model version.
#[component]
pub fn CheckCircleOutlined() -> impl IntoView {
    icon(&CHECK_CIRCLE_OUTLINED)
}

const MINUS_CIRCLE_OUTLINED: Glyph = Glyph {
    view_box: "64 64 896 896",
    even_odd: false,
    paths: &[
        "M696 480H328c-4.4 0-8 3.6-8 8v48c0 4.4 3.6 8 8 8h368c4.4 0 8-3.6 8-8v-48c0-4.4-3.6-8-8-8z",
        "M512 64C264.6 64 64 264.6 64 512s200.6 448 448 448 448-200.6 448-448S759.4 64 512 64zm0 820c-205.4 0-372-166.6-372-372s166.6-372 372-372 372 166.6 372 372-166.6 372-372 372z",
    ],
};

/// A model version whose registration is pending.
#[component]
pub fn MinusCircleOutlined() -> impl IntoView {
    icon(&MINUS_CIRCLE_OUTLINED)
}

const CLOSE_CIRCLE_OUTLINED: Glyph = Glyph {
    view_box: "64 64 896 896",
    even_odd: true,
    paths: &[
        "M512 64c247.4 0 448 200.6 448 448S759.4 960 512 960 64 759.4 64 512 264.6 64 512 64zm0 76c-205.4 0-372 166.6-372 372s166.6 372 372 372 372-166.6 372-372-166.6-372-372-372zm128.01 198.83c.03 0 .05.01.09.06l45.02 45.01a.2.2 0 01.05.09.12.12 0 010 .07c0 .02-.01.04-.05.08L557.25 512l127.87 127.86a.27.27 0 01.05.06v.02a.12.12 0 010 .07c0 .03-.01.05-.05.09l-45.02 45.02a.2.2 0 01-.09.05.12.12 0 01-.07 0c-.02 0-.04-.01-.08-.05L512 557.25 384.14 685.12c-.04.04-.06.05-.08.05a.12.12 0 01-.07 0c-.03 0-.05-.01-.09-.05l-45.02-45.02a.2.2 0 01-.05-.09.12.12 0 010-.07c0-.02.01-.04.06-.08L466.75 512 338.88 384.14a.27.27 0 01-.05-.06l-.01-.02a.12.12 0 010-.07c0-.03.01-.05.05-.09l45.02-45.02a.2.2 0 01.09-.05.12.12 0 01.07 0c.02 0 .04.01.08.06L512 466.75l127.86-127.86c.04-.05.06-.06.08-.06a.12.12 0 01.07 0z",
    ],
};

/// A model version whose registration failed.
#[component]
pub fn CloseCircleOutlined() -> impl IntoView {
    icon(&CLOSE_CIRCLE_OUTLINED)
}

const USER_OUTLINED: Glyph = Glyph {
    view_box: "64 64 896 896",
    even_odd: false,
    paths: &[
        "M858.5 763.6a374 374 0 00-80.6-119.5 375.63 375.63 0 00-119.5-80.6c-.4-.2-.8-.3-1.2-.5C719.5 518 760 444.7 760 362c0-137-111-248-248-248S264 225 264 362c0 82.7 40.5 156 102.8 201.1-.4.2-.8.3-1.2.5-44.8 18.9-85 46-119.5 80.6a375.63 375.63 0 00-80.6 119.5A371.7 371.7 0 00136 901.8a8 8 0 008 8.2h60c4.4 0 7.9-3.5 8-7.8 2-77.2 33-149.5 87.8-204.3 56.7-56.7 132-87.9 212.2-87.9s155.5 31.2 212.2 87.9C779 752.7 810 825 812 902.2c.1 4.4 3.6 7.8 8 7.8h60a8 8 0 008-8.2c-1-47.8-10.9-94.3-29.5-138.2zM512 534c-45.9 0-89.1-17.9-121.6-50.4S340 407.9 340 362c0-45.9 17.9-89.1 50.4-121.6S466.1 190 512 190s89.1 17.9 121.6 50.4S684 316.1 684 362c0 45.9-17.9 89.1-50.4 121.6S557.9 534 512 534z",
    ],
};

/// The signed-in user.
#[component]
pub fn UserOutlined() -> impl IntoView {
    icon(&USER_OUTLINED)
}

const DOWN_OUTLINED: Glyph = Glyph {
    view_box: "64 64 896 896",
    even_odd: false,
    paths: &[
        "M884 256h-75c-5.1 0-9.9 2.5-12.9 6.6L512 654.2 227.9 262.6c-3-4.1-7.8-6.6-12.9-6.6h-75c-6.5 0-10.3 7.4-6.5 12.7l352.6 486.1c12.8 17.6 39 17.6 51.7 0l352.6-486.1c3.9-5.3.1-12.7-6.4-12.7z",
    ],
};

/// The switcher of a tree node: turned for a closed node.
#[component]
pub fn DownOutlined() -> impl IntoView {
    icon(&DOWN_OUTLINED)
}

const LOADING_OUTLINED: Glyph = Glyph {
    view_box: "0 0 1024 1024",
    even_odd: false,
    paths: &[
        "M988 548c-19.9 0-36-16.1-36-36 0-59.4-11.6-117-34.6-171.3a440.45 440.45 0 00-94.3-139.9 437.71 437.71 0 00-139.9-94.3C629 83.6 571.4 72 512 72c-19.9 0-36-16.1-36-36s16.1-36 36-36c69.1 0 136.2 13.5 199.3 40.3C772.3 66 827 103 874 150c47 47 83.9 101.8 109.7 162.7 26.7 63.1 40.2 130.2 40.2 199.3.1 19.9-16 36-35.9 36z",
    ],
};

/// A button's spinner.
#[component]
pub fn LoadingOutlined() -> impl IntoView {
    icon(&LOADING_OUTLINED)
}

const CLOSE_OUTLINED: Glyph = Glyph {
    view_box: "64 64 896 896",
    even_odd: true,
    paths: &[
        "M799.86 166.31c.02 0 .04.02.08.06l57.69 57.7c.04.03.05.05.06.08a.12.12 0 010 .06c0 .03-.02.05-.06.09L569.93 512l287.7 287.7c.04.04.05.06.06.09a.12.12 0 010 .07c0 .02-.02.04-.06.08l-57.7 57.69c-.03.04-.05.05-.07.06a.12.12 0 01-.07 0c-.03 0-.05-.02-.09-.06L512 569.93l-287.7 287.7c-.04.04-.06.05-.09.06a.12.12 0 01-.07 0c-.02 0-.04-.02-.08-.06l-57.69-57.7c-.04-.03-.05-.05-.06-.07a.12.12 0 010-.07c0-.03.02-.05.06-.09L454.07 512l-287.7-287.7c-.04-.04-.05-.06-.06-.09a.12.12 0 010-.07c0-.02.02-.04.06-.08l57.7-57.69c.03-.04.05-.05.07-.06a.12.12 0 01.07 0c.03 0 .05.02.09.06L512 454.07l287.7-287.7c.04-.04.06-.05.09-.06a.12.12 0 01.07 0z",
    ],
};

/// Close a modal or a toast.
#[component]
pub fn CloseOutlined() -> impl IntoView {
    icon(&CLOSE_OUTLINED)
}

const CHECK_CIRCLE_FILLED: Glyph = Glyph {
    view_box: "64 64 896 896",
    even_odd: false,
    paths: &[
        "M512 64C264.6 64 64 264.6 64 512s200.6 448 448 448 448-200.6 448-448S759.4 64 512 64zm193.5 301.7l-210.6 292a31.8 31.8 0 01-51.7 0L318.5 484.9c-3.8-5.3 0-12.7 6.5-12.7h46.9c10.2 0 19.9 4.9 25.9 13.3l71.2 98.8 157.2-218c6-8.3 15.6-13.3 25.9-13.3H699c6.5 0 10.3 7.4 6.5 12.7z",
    ],
};

/// A success toast, and the new access token.
#[component]
pub fn CheckCircleFilled() -> impl IntoView {
    icon(&CHECK_CIRCLE_FILLED)
}

const CLOSE_CIRCLE_FILLED: Glyph = Glyph {
    view_box: "64 64 896 896",
    even_odd: true,
    paths: &[
        "M512 64c247.4 0 448 200.6 448 448S759.4 960 512 960 64 759.4 64 512 264.6 64 512 64zm127.98 274.82h-.04l-.08.06L512 466.75 384.14 338.88c-.04-.05-.06-.06-.08-.06a.12.12 0 00-.07 0c-.03 0-.05.01-.09.05l-45.02 45.02a.2.2 0 00-.05.09.12.12 0 000 .07v.02a.27.27 0 00.06.06L466.75 512 338.88 639.86c-.05.04-.06.06-.06.08a.12.12 0 000 .07c0 .03.01.05.05.09l45.02 45.02a.2.2 0 00.09.05.12.12 0 00.07 0c.02 0 .04-.01.08-.05L512 557.25l127.86 127.87c.04.04.06.05.08.05a.12.12 0 00.07 0c.03 0 .05-.01.09-.05l45.02-45.02a.2.2 0 00.05-.09.12.12 0 000-.07v-.02a.27.27 0 00-.05-.06L557.25 512l127.87-127.86c.04-.04.05-.06.05-.08a.12.12 0 000-.07c0-.03-.01-.05-.05-.09l-45.02-45.02a.2.2 0 00-.09-.05.12.12 0 00-.07 0z",
    ],
};

/// An error toast.
#[component]
pub fn CloseCircleFilled() -> impl IntoView {
    icon(&CLOSE_CIRCLE_FILLED)
}

const COPY_OUTLINED: Glyph = Glyph {
    view_box: "64 64 896 896",
    even_odd: false,
    paths: &[
        "M832 64H296c-4.4 0-8 3.6-8 8v56c0 4.4 3.6 8 8 8h496v688c0 4.4 3.6 8 8 8h56c4.4 0 8-3.6 8-8V96c0-17.7-14.3-32-32-32zM704 192H192c-17.7 0-32 14.3-32 32v530.7c0 8.5 3.4 16.6 9.4 22.6l173.3 173.3c2.2 2.2 4.7 4 7.4 5.5v1.9h4.2c3.5 1.3 7.2 2 11 2H704c17.7 0 32-14.3 32-32V224c0-17.7-14.3-32-32-32zM350 856.2L263.9 770H350v86.2zM664 888H414V746c0-22.1-17.9-40-40-40H232V264h432v624z",
    ],
};

/// Copy a new access token.
#[component]
pub fn CopyOutlined() -> impl IntoView {
    icon(&COPY_OUTLINED)
}

const CHECK_OUTLINED: Glyph = Glyph {
    view_box: "64 64 896 896",
    even_odd: false,
    paths: &[
        "M912 190h-69.9c-9.8 0-19.1 4.5-25.1 12.2L404.7 724.5 207 474a32 32 0 00-25.1-12.2H112c-6.7 0-10.4 7.7-6.3 12.9l273.9 347c12.8 16.2 37.4 16.2 50.3 0l488.4-618.9c4.1-5.1.4-12.8-6.3-12.8z",
    ],
};

/// The token is copied.
#[component]
pub fn CheckOutlined() -> impl IntoView {
    icon(&CHECK_OUTLINED)
}

const LEFT_OUTLINED: Glyph = Glyph {
    view_box: "64 64 896 896",
    even_odd: false,
    paths: &[
        "M724 218.3V141c0-6.7-7.7-10.4-12.9-6.3L260.3 486.8a31.86 31.86 0 000 50.3l450.8 352.1c5.3 4.1 12.9.4 12.9-6.3v-77.3c0-4.9-2.3-9.6-6.1-12.6l-360-281 360-281.1c3.8-3 6.1-7.7 6.1-12.6z",
    ],
};

/// The previous page.
#[component]
pub fn LeftOutlined() -> impl IntoView {
    icon(&LEFT_OUTLINED)
}

const RIGHT_OUTLINED: Glyph = Glyph {
    view_box: "64 64 896 896",
    even_odd: false,
    paths: &[
        "M765.7 486.8L314.9 134.7A7.97 7.97 0 00302 141v77.3c0 4.9 2.3 9.6 6.1 12.6l360 281.1-360 281.1c-3.9 3-6.1 7.7-6.1 12.6V883c0 6.7 7.7 10.4 12.9 6.3l450.8-352.1a31.96 31.96 0 000-50.4z",
    ],
};

/// The next page.
#[component]
pub fn RightOutlined() -> impl IntoView {
    icon(&RIGHT_OUTLINED)
}

const DOUBLE_LEFT_OUTLINED: Glyph = Glyph {
    view_box: "64 64 896 896",
    even_odd: false,
    paths: &[
        "M272.9 512l265.4-339.1c4.1-5.2.4-12.9-6.3-12.9h-77.3c-4.9 0-9.6 2.3-12.6 6.1L186.8 492.3a31.99 31.99 0 000 39.5l255.3 326.1c3 3.9 7.7 6.1 12.6 6.1H532c6.7 0 10.4-7.7 6.3-12.9L272.9 512zm304 0l265.4-339.1c4.1-5.2.4-12.9-6.3-12.9h-77.3c-4.9 0-9.6 2.3-12.6 6.1L490.8 492.3a31.99 31.99 0 000 39.5l255.3 326.1c3 3.9 7.7 6.1 12.6 6.1H836c6.7 0 10.4-7.7 6.3-12.9L576.9 512z",
    ],
};

/// Five pages back.
#[component]
pub fn DoubleLeftOutlined() -> impl IntoView {
    icon(&DOUBLE_LEFT_OUTLINED)
}

const DOUBLE_RIGHT_OUTLINED: Glyph = Glyph {
    view_box: "64 64 896 896",
    even_odd: false,
    paths: &[
        "M533.2 492.3L277.9 166.1c-3-3.9-7.7-6.1-12.6-6.1H188c-6.7 0-10.4 7.7-6.3 12.9L447.1 512 181.7 851.1A7.98 7.98 0 00188 864h77.3c4.9 0 9.6-2.3 12.6-6.1l255.3-326.1c9.1-11.7 9.1-27.9 0-39.5zm304 0L581.9 166.1c-3-3.9-7.7-6.1-12.6-6.1H492c-6.7 0-10.4 7.7-6.3 12.9L751.1 512 485.7 851.1A7.98 7.98 0 00492 864h77.3c4.9 0 9.6-2.3 12.6-6.1l255.3-326.1c9.1-11.7 9.1-27.9 0-39.5z",
    ],
};

/// Five pages on.
#[component]
pub fn DoubleRightOutlined() -> impl IntoView {
    icon(&DOUBLE_RIGHT_OUTLINED)
}

const EXPORT_OUTLINED: Glyph = Glyph {
    view_box: "64 64 896 896",
    even_odd: true,
    paths: &[
        "M880 912H144c-17.7 0-32-14.3-32-32V144c0-17.7 14.3-32 32-32h360c4.4 0 8 3.6 8 8v56c0 4.4-3.6 8-8 8H184v656h656V520c0-4.4 3.6-8 8-8h56c4.4 0 8 3.6 8 8v360c0 17.7-14.3 32-32 32zM770.87 199.13l-52.2-52.2a8.01 8.01 0 014.7-13.6l179.4-21c5.1-.6 9.5 3.7 8.9 8.9l-21 179.4c-.8 6.6-8.9 9.4-13.6 4.7l-52.4-52.4-256.2 256.2a8.03 8.03 0 01-11.3 0l-42.4-42.4a8.03 8.03 0 010-11.3l256.1-256.3z",
    ],
};

/// A link that opens in a new tab.
#[component]
pub fn ExportOutlined() -> impl IntoView {
    icon(&EXPORT_OUTLINED)
}
