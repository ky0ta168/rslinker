// mathlib.c — mathlib.dll としてビルドされる DLL
// power() をエクスポートし、main.obj から dllimport で呼び出される。

__declspec(dllexport) int power(int base, int exp) {
    int result = 1;
    for (int i = 0; i < exp; i++)
        result *= base;
    return result;
}
