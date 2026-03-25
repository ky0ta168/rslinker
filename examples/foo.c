/* DLL インポートを明示的に使うサンプル */
__declspec(dllimport) int __cdecl printf(const char *, ...);
__declspec(dllimport) void __stdcall ExitProcess(unsigned int uExitCode);

int g_value = 42;

int add(int a, int b) {
    printf("add(%d, %d) = %d\n", a, b, a + b);
    return a + b;
}

/* エントリポイント: main → COFF シンボル _main (LinkerOptions::entry_point と一致) */
int main(void) {
    printf("g_value = %d\n", g_value);
    add(1, 2);
    ExitProcess(0);
    return 0;
}
