// jmplib.c — jmplib.dll としてビルドされる DLL
// add_from_dll / mul_from_dll の 2 関数をエクスポートする。
// main.obj 側でどちらの呼び出し方法を使うかによって、
// リンカが IAT 直接参照か .dlljmp スタブかを選択する。

__declspec(dllexport) int add_from_dll(int a, int b) {
    return a + b;
}

__declspec(dllexport) int mul_from_dll(int a, int b) {
    return a * b;
}
