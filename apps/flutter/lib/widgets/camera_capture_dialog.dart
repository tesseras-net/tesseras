import 'package:image_picker/image_picker.dart';

/// Cross-platform camera capture using image_picker.
///
/// On mobile (Android/iOS), opens the native camera app.
/// On desktop (Linux/macOS/Windows), opens the system file picker
/// for selecting an existing image (no live preview).
class CameraCaptureDialog {
  CameraCaptureDialog._();

  static final _picker = ImagePicker();

  /// Captures or picks a photo and returns the file path, or null.
  static Future<String?> show(dynamic context) async {
    final image = await _picker.pickImage(
      source: ImageSource.camera,
      maxWidth: 1024,
      maxHeight: 1024,
      imageQuality: 90,
    );
    return image?.path;
  }
}
