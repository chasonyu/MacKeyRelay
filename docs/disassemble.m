#import <Foundation/Foundation.h>
#import <objc/runtime.h>
#import <dlfcn.h>
#import <mach/mach.h>
#import <mach/mach_vm.h>
typedef void* (*Create)(const char*,void*,int,void*,void*);
typedef size_t (*Disasm)(void*,uint8_t*,uint64_t,uint64_t,char*,size_t);
static BOOL readMem(uint64_t a,void *b,size_t n){mach_vm_size_t got=0;return mach_vm_read_overwrite(mach_task_self(),a,n,(mach_vm_address_t)b,&got)==0&&got==n;}
static void describe(uint64_t a){Dl_info d={0};if(dladdr((void*)a,&d)&&d.dli_sname)printf(" <%s>",d.dli_sname);}
int main(){@autoreleasepool{
 dlopen("/System/Library/PrivateFrameworks/ScreenSharing.framework/ScreenSharing",RTLD_LAZY);
 dlopen("/System/Library/PrivateFrameworks/ScreenSharing.framework/Versions/A/Frameworks/ScreenSharingUI.framework/ScreenSharingUI",RTLD_LAZY);
 void *l=dlopen("/Applications/Xcode.app/Contents/Developer/Toolchains/XcodeDefault.xctoolchain/usr/lib/libLTO.dylib",RTLD_LAZY);
 void (*initialize)(void) = dlsym(l,"lto_initialize_disassembler"); if(initialize) initialize();
 Create create=(Create)dlsym(l,"LLVMCreateDisasm");Disasm dis=(Disasm)dlsym(l,"LLVMDisasmInstruction");
 if(!create||!dis){puts("No LLVM disassembler");return 1;}void *ctx=create("arm64-apple-macos",NULL,0,NULL,NULL);if(!ctx){puts("No target");return 2;}
 const char *classes[]={"SSApplication","SSApplication","SSApplication","SSSessionView","SSSessionView","SSEventHelperManager","SSEventHelperManager","SSKeyboardEvent","SSEventSession","SSSession","SSSessionView","SSFrameBufferView","SSFrameBufferView","SSApplication","C"};
 const char *sels[]={"sendEvent:","sendChangedModifierFlags:","ssSetInputEventConsumer:","windowDidBecomeKey:","windowDidResignKey:","captureSpecialKeys:","ssSetInputEventConsumer:","initWithKeyCode:withState:withEvent:","stSendKeyboardEvent:","stSendKeyboardEvent:","configureInputEventConsumer","keyDown:","keyUp:","sendChangedModifierFlags:","SSSendChangedModifierFlags"};
 for(int k=0;k<15;k++){
  Method m=class_getInstanceMethod(objc_getClass(classes[k]),sel_registerName(sels[k]));
  uint8_t *p=m?(uint8_t*)method_getImplementation(m):(uint8_t*)dlsym(RTLD_DEFAULT,sels[k]);if(!p)continue;Dl_info d;dladdr(p,&d);printf("\nMETHOD %s %s image=%s offset=0x%lx\n",classes[k],sels[k],d.dli_fname,(unsigned long)(p-(uint8_t*)d.dli_fbase));
  if(!strcmp(sels[k],"SSSendChangedModifierFlags")) {
    uint32_t ins;memcpy(&ins,p+0x30,4);
    int64_t imm=(((ins>>5)&0x7ffff)<<2)|((ins>>29)&3);if(imm&(1<<20))imm-=1<<21;
    uint64_t table=(((uint64_t)p+0x30)&~4095ULL)+(imm<<12)+2152;
    puts("MODIFIER_TABLE mask -> keyCode");
    for(int r=0;r<10;r++){uint64_t pair[2];if(readMem(table+r*16,pair,16))printf("0x%llx -> %llu (0x%llx)\n",pair[0],pair[1],pair[1]);}
  }
  uint64_t regs[32]={0};
  for(int i=0;i<8192;i+=4){Dl_info now;dladdr(p+i,&now);if(i&&now.dli_saddr!=d.dli_saddr)break;
   char text[512];size_t n=dis(ctx,p+i,4,(uint64_t)p+i,text,sizeof text);if(!n)break;
   printf("+0x%04x %s",i,text);uint32_t ins;memcpy(&ins,p+i,4);
   if((ins&0x7c000000)==0x14000000){int64_t imm=((int64_t)(int32_t)(ins<<6))>>4;describe((uint64_t)p+i+imm);}
   if((ins&0x9f000000)==0x90000000){int64_t imm=(((ins>>5)&0x7ffff)<<2)|((ins>>29)&3);if(imm&(1<<20))imm-=1<<21;regs[ins&31]=(((uint64_t)p+i)&~4095ULL)+(imm<<12);}
   else if((ins&0xffc00000)==0xf9400000){unsigned rt=ins&31,rn=(ins>>5)&31;uint64_t a=regs[rn]+(((ins>>10)&4095)*8),v=0;if(regs[rn]&&readMem(a,&v,8)){printf(" [slot]");describe(a);describe(v);char str[160]={0};if(readMem(v,str,159)){BOOL ok=YES;int j=0;for(;j<159&&str[j];j++)if(str[j]<32||str[j]>126){ok=NO;break;}if(ok&&j>1&&j<159)printf(" string=%s",str);}regs[rt]=v;}else regs[rt]=0;}
   else if((ins&0xff000000)==0x91000000){unsigned rd=ins&31,rn=(ins>>5)&31;regs[rd]=regs[rn]+((ins>>10)&4095);}
   printf("\n");
  }
 }
}}
