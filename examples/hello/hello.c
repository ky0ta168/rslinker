#include <windows.h>

// GCC は main() の先頭に __main() の呼び出しを挿入する。
// MinGW CRT なしでリンクするためダミーで定義する。
void __main(void) {}

int add(int x, int y) {
    return x + y;
}

int x = 77;

int main(void) {
    int result = add(x, 23);
    char num[16];
    _itoa(result, num, 10);
    char msg[64] = "Hello from rslinker!\nresult = ";
    strcat(msg, num);
    MessageBoxA(NULL, msg, "rslinker Test", MB_OK);
    ExitProcess(0);
    return 0;
}
