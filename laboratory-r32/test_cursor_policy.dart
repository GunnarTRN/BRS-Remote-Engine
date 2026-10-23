import '../flutter/lib/lab_cursor_policy.dart';
void main() {
  var count = 0;
  for (final moved in [false, true]) {
    for (final show in [false, true]) {
      for (final image in [false, true]) {
        final hidden = labHideLocalCursor(remoteMoved: moved, showRemote: show, hasRemoteImage: image);
        if (hidden != (moved && show && image)) throw StateError('Cursor visibility regression');
        count++;
      }
    }
  }
  print('LAB_CURSOR_CASES_PASSED: $count');
}
