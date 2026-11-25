//! h264ify MediaSource and canPlayType interception shim.

pub const H264IFY_SHIM_JS: &str = r#"
(function() {
    'use strict';
    var BLOCKED_CODECS = /vp8|vp9|vp09|av01|av1/i;
    var WEBM_VIDEO = new RegExp('^video/webm', 'i');

    if (window.HTMLMediaElement && window.HTMLMediaElement.prototype) {
        var origCanPlay = window.HTMLMediaElement.prototype.canPlayType;
        window.HTMLMediaElement.prototype.canPlayType = function(type) {
            if (typeof type === 'string') {
                if (BLOCKED_CODECS.test(type) || WEBM_VIDEO.test(type)) {
                    return '';
                }
                if (/avc1/i.test(type)) {
                    var r = origCanPlay.call(this, type);
                    return r ? r : 'probably';
                }
            }
            return origCanPlay.call(this, type);
        };
    }

    function patchMediaSource(ms) {
        if (!ms || !ms.isTypeSupported) return;
        var origIsSupported = ms.isTypeSupported.bind(ms);
        ms.isTypeSupported = function(type) {
            if (typeof type === 'string') {
                if (BLOCKED_CODECS.test(type) || WEBM_VIDEO.test(type)) {
                    return false;
                }
                if (/avc1/i.test(type)) {
                    return true;
                }
            }
            return origIsSupported(type);
        };
    }

    if (window.MediaSource) {
        patchMediaSource(window.MediaSource);
    }
    if (window.WebKitMediaSource) {
        patchMediaSource(window.WebKitMediaSource);
    }
})();
"#;

/// Returns the initialization script string if h264ify is enabled.
pub fn get_initialization_script(enabled: bool) -> Option<&'static str> {
    if enabled {
        Some(H264IFY_SHIM_JS)
    } else {
        None
    }
}
