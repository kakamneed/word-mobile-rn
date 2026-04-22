#import "WordCoreModule.h"

#import <Foundation/Foundation.h>
#import <dlfcn.h>

static NSString *const WordCoreModuleErrorCode = @"E_WORDCORE_NOT_IMPLEMENTED";
static NSString *const WordCoreRustNotLinkedCode = @"E_WORDCORE_RUST_NOT_LINKED";
static NSString *const WordCoreRustRuntimeCode = @"E_WORDCORE_RUST_RUNTIME";
static NSString *const WordCoreRustErrorPrefix = @"__WORDMOBILE_ERROR__:";

typedef char *(*WordMobileRustInitFn)(const char *app_data_dir, const char *app_config_dir, const char *app_cache_dir, const char *bundle_resource_dir);
typedef char *(*WordMobileRustStringFn)(void);
typedef char *(*WordMobileRustStringArgFn)(const char *value);
typedef char *(*WordMobileRustIntBoolFn)(long long value, signed char flag);
typedef void (*WordMobileRustFreeFn)(char *value);

static BOOL WordCoreRustRuntimeInitialized = NO;

@implementation WordCoreModule

RCT_EXPORT_MODULE(WordCoreModule)

+ (BOOL)requiresMainQueueSetup
{
  return NO;
}

- (void *)resolveRustSymbol:(const char *)symbolName
{
  return dlsym(RTLD_DEFAULT, symbolName);
}

- (void)freeRustStringIfNeeded:(char *)value
{
  if (value == NULL) {
    return;
  }

  WordMobileRustFreeFn freeFn = (WordMobileRustFreeFn)[self resolveRustSymbol:"word_mobile_ios_string_free"];
  if (freeFn != NULL) {
    freeFn(value);
  }
}

- (NSError *)errorWithCode:(NSString *)code description:(NSString *)description
{
  return [NSError errorWithDomain:@"WordCoreModule" code:1 userInfo:@{
    NSLocalizedDescriptionKey: description,
    @"code": code,
  }];
}

- (NSString *)consumeRustResult:(char *)rawValue error:(NSError **)error
{
  if (rawValue == NULL) {
    if (error != NULL) {
      *error = [self errorWithCode:WordCoreRustRuntimeCode description:@"Rust iOS bridge returned a null pointer."];
    }
    return nil;
  }

  NSString *result = [[NSString alloc] initWithUTF8String:rawValue];
  [self freeRustStringIfNeeded:rawValue];

  if (result == nil) {
    if (error != NULL) {
      *error = [self errorWithCode:WordCoreRustRuntimeCode description:@"Rust iOS bridge returned invalid UTF-8."];
    }
    return nil;
  }

  if ([result hasPrefix:WordCoreRustErrorPrefix]) {
    if (error != NULL) {
      NSString *message = [result substringFromIndex:WordCoreRustErrorPrefix.length];
      *error = [self errorWithCode:WordCoreRustRuntimeCode description:message];
    }
    return nil;
  }

  return result;
}

- (NSURL *)applicationSupportURL:(NSError **)error
{
  NSFileManager *fileManager = [NSFileManager defaultManager];
  NSURL *baseURL = [fileManager URLForDirectory:NSApplicationSupportDirectory inDomain:NSUserDomainMask appropriateForURL:nil create:YES error:error];
  if (baseURL == nil) {
    return nil;
  }
  return [baseURL URLByAppendingPathComponent:@"WordMobile" isDirectory:YES];
}

- (NSURL *)configURLFromSupportURL:(NSURL *)supportURL
{
  return [supportURL URLByAppendingPathComponent:@"config" isDirectory:YES];
}

- (NSURL *)cacheURL:(NSError **)error
{
  NSFileManager *fileManager = [NSFileManager defaultManager];
  NSURL *baseURL = [fileManager URLForDirectory:NSCachesDirectory inDomain:NSUserDomainMask appropriateForURL:nil create:YES error:error];
  if (baseURL == nil) {
    return nil;
  }
  return [baseURL URLByAppendingPathComponent:@"WordMobile" isDirectory:YES];
}

- (BOOL)ensureDirectory:(NSURL *)url excludeFromBackup:(BOOL)excludeFromBackup error:(NSError **)error
{
  NSFileManager *fileManager = [NSFileManager defaultManager];
  if (![fileManager createDirectoryAtURL:url withIntermediateDirectories:YES attributes:nil error:error]) {
    return NO;
  }
  if (excludeFromBackup) {
    NSError *resourceError = nil;
    if (![url setResourceValue:@YES forKey:NSURLIsExcludedFromBackupKey error:&resourceError] && error != NULL && *error == nil) {
      *error = resourceError;
      return NO;
    }
  }
  return YES;
}

- (BOOL)ensureRustRuntimeInitialized:(NSError **)error
{
  @synchronized ([WordCoreModule class]) {
    if (WordCoreRustRuntimeInitialized) {
      return YES;
    }

    WordMobileRustInitFn initFn = (WordMobileRustInitFn)[self resolveRustSymbol:"word_mobile_ios_initialize"];
    if (initFn == NULL) {
      if (error != NULL) {
        *error = [self errorWithCode:WordCoreRustNotLinkedCode description:@"Rust iOS symbols are not linked into the current target yet."];
      }
      return NO;
    }

    NSURL *supportURL = [self applicationSupportURL:error];
    if (supportURL == nil) {
      return NO;
    }
    NSURL *configURL = [self configURLFromSupportURL:supportURL];
    NSURL *cacheURL = [self cacheURL:error];
    if (cacheURL == nil) {
      return NO;
    }
    NSString *bundleResourcePath = [[NSBundle mainBundle] resourcePath];
    if (bundleResourcePath.length == 0) {
      if (error != NULL) {
        *error = [self errorWithCode:WordCoreRustRuntimeCode description:@"Bundle resource path is unavailable."];
      }
      return NO;
    }

    if (![self ensureDirectory:supportURL excludeFromBackup:NO error:error]) {
      return NO;
    }
    if (![self ensureDirectory:configURL excludeFromBackup:YES error:error]) {
      return NO;
    }
    if (![self ensureDirectory:cacheURL excludeFromBackup:YES error:error]) {
      return NO;
    }

    char *rawResult = initFn(
      supportURL.path.UTF8String,
      configURL.path.UTF8String,
      cacheURL.path.UTF8String,
      bundleResourcePath.UTF8String
    );
    NSString *result = [self consumeRustResult:rawResult error:error];
    if (result == nil) {
      return NO;
    }

    WordCoreRustRuntimeInitialized = YES;
    return YES;
  }
}

- (void)rejectWithNSError:(NSError *)error rejecter:(RCTPromiseRejectBlock)reject
{
  NSString *code = error.userInfo[@"code"] ?: WordCoreRustRuntimeCode;
  reject(code, error.localizedDescription, error);
}

- (void)resolveWithRustStringSymbol:(const char *)symbolName resolver:(RCTPromiseResolveBlock)resolve rejecter:(RCTPromiseRejectBlock)reject
{
  NSError *runtimeError = nil;
  if (![self ensureRustRuntimeInitialized:&runtimeError]) {
    [self rejectWithNSError:runtimeError rejecter:reject];
    return;
  }

  WordMobileRustStringFn function = (WordMobileRustStringFn)[self resolveRustSymbol:symbolName];
  if (function == NULL) {
    NSError *symbolError = [self errorWithCode:WordCoreRustNotLinkedCode description:[NSString stringWithFormat:@"Rust symbol %s is not linked into the current target yet.", symbolName]];
    [self rejectWithNSError:symbolError rejecter:reject];
    return;
  }

  NSError *callError = nil;
  NSString *result = [self consumeRustResult:function() error:&callError];
  if (result == nil) {
    [self rejectWithNSError:callError rejecter:reject];
    return;
  }

  resolve(result);
}

- (void)resolveWithRustVoidSymbol:(const char *)symbolName resolver:(RCTPromiseResolveBlock)resolve rejecter:(RCTPromiseRejectBlock)reject
{
  [self resolveWithRustStringSymbol:symbolName resolver:^(__unused id payload) {
    resolve(nil);
  } rejecter:reject];
}

- (void)resolveWithRustStringArgSymbol:(const char *)symbolName value:(NSString *)value resolver:(RCTPromiseResolveBlock)resolve rejecter:(RCTPromiseRejectBlock)reject
{
  NSError *runtimeError = nil;
  if (![self ensureRustRuntimeInitialized:&runtimeError]) {
    [self rejectWithNSError:runtimeError rejecter:reject];
    return;
  }

  WordMobileRustStringArgFn function = (WordMobileRustStringArgFn)[self resolveRustSymbol:symbolName];
  if (function == NULL) {
    NSError *symbolError = [self errorWithCode:WordCoreRustNotLinkedCode description:[NSString stringWithFormat:@"Rust symbol %s is not linked into the current target yet.", symbolName]];
    [self rejectWithNSError:symbolError rejecter:reject];
    return;
  }

  NSError *callError = nil;
  NSString *result = [self consumeRustResult:function(value.UTF8String) error:&callError];
  if (result == nil) {
    [self rejectWithNSError:callError rejecter:reject];
    return;
  }

  resolve(result);
}

- (void)resolveWithRustVoidStringArgSymbol:(const char *)symbolName value:(NSString *)value resolver:(RCTPromiseResolveBlock)resolve rejecter:(RCTPromiseRejectBlock)reject
{
  [self resolveWithRustStringArgSymbol:symbolName value:value resolver:^(__unused id payload) {
    resolve(nil);
  } rejecter:reject];
}

- (void)resolveWithRustIntBoolSymbol:(const char *)symbolName integerValue:(long long)integerValue boolValue:(BOOL)boolValue resolver:(RCTPromiseResolveBlock)resolve rejecter:(RCTPromiseRejectBlock)reject
{
  NSError *runtimeError = nil;
  if (![self ensureRustRuntimeInitialized:&runtimeError]) {
    [self rejectWithNSError:runtimeError rejecter:reject];
    return;
  }

  WordMobileRustIntBoolFn function = (WordMobileRustIntBoolFn)[self resolveRustSymbol:symbolName];
  if (function == NULL) {
    NSError *symbolError = [self errorWithCode:WordCoreRustNotLinkedCode description:[NSString stringWithFormat:@"Rust symbol %s is not linked into the current target yet.", symbolName]];
    [self rejectWithNSError:symbolError rejecter:reject];
    return;
  }

  NSError *callError = nil;
  NSString *result = [self consumeRustResult:function(integerValue, boolValue ? 1 : 0) error:&callError];
  if (result == nil) {
    [self rejectWithNSError:callError rejecter:reject];
    return;
  }

  resolve(nil);
}

- (void)rejectNotImplemented:(NSString *)method rejecter:(RCTPromiseRejectBlock)reject
{
  NSString *message = [NSString stringWithFormat:@"%@ is not implemented on iOS yet.", method];
  reject(WordCoreModuleErrorCode, message, nil);
}

RCT_EXPORT_METHOD(getBootstrapState:(RCTPromiseResolveBlock)resolve rejecter:(RCTPromiseRejectBlock)reject)
{
  [self resolveWithRustStringSymbol:"word_mobile_ios_get_bootstrap_state" resolver:resolve rejecter:reject];
}

RCT_EXPORT_METHOD(markOnboardingCompleted:(RCTPromiseResolveBlock)resolve rejecter:(RCTPromiseRejectBlock)reject)
{
  [self resolveWithRustVoidSymbol:"word_mobile_ios_mark_onboarding_completed" resolver:resolve rejecter:reject];
}

RCT_EXPORT_METHOD(getTodayHomeState:(RCTPromiseResolveBlock)resolve rejecter:(RCTPromiseRejectBlock)reject)
{
  [self resolveWithRustStringSymbol:"word_mobile_ios_get_today_home_state" resolver:resolve rejecter:reject];
}

RCT_EXPORT_METHOD(getSettings:(RCTPromiseResolveBlock)resolve rejecter:(RCTPromiseRejectBlock)reject)
{
  [self resolveWithRustStringSymbol:"word_mobile_ios_get_settings" resolver:resolve rejecter:reject];
}

RCT_EXPORT_METHOD(getActivePlan:(RCTPromiseResolveBlock)resolve rejecter:(RCTPromiseRejectBlock)reject)
{
  [self resolveWithRustStringSymbol:"word_mobile_ios_get_active_plan" resolver:resolve rejecter:reject];
}

RCT_EXPORT_METHOD(savePlan:(NSString *)requestJson resolver:(RCTPromiseResolveBlock)resolve rejecter:(RCTPromiseRejectBlock)reject)
{
  [self resolveWithRustStringArgSymbol:"word_mobile_ios_save_plan" value:requestJson resolver:resolve rejecter:reject];
}

RCT_EXPORT_METHOD(applySavedPlanToToday:(RCTPromiseResolveBlock)resolve rejecter:(RCTPromiseRejectBlock)reject)
{
  [self resolveWithRustStringSymbol:"word_mobile_ios_apply_saved_plan_to_today" resolver:resolve rejecter:reject];
}

RCT_EXPORT_METHOD(getWordbooks:(RCTPromiseResolveBlock)resolve rejecter:(RCTPromiseRejectBlock)reject)
{
  [self resolveWithRustStringSymbol:"word_mobile_ios_get_wordbooks" resolver:resolve rejecter:reject];
}

RCT_EXPORT_METHOD(toggleWordbook:(double)wordbookId isActive:(BOOL)isActive resolver:(RCTPromiseResolveBlock)resolve rejecter:(RCTPromiseRejectBlock)reject)
{
  [self resolveWithRustIntBoolSymbol:"word_mobile_ios_toggle_wordbook" integerValue:(long long)wordbookId boolValue:isActive resolver:resolve rejecter:reject];
}

RCT_EXPORT_METHOD(startStudySession:(NSString *)requestJson resolver:(RCTPromiseResolveBlock)resolve rejecter:(RCTPromiseRejectBlock)reject)
{
  [self resolveWithRustStringArgSymbol:"word_mobile_ios_start_study_session" value:requestJson resolver:resolve rejecter:reject];
}

RCT_EXPORT_METHOD(submitStudyAnswer:(NSString *)requestJson resolver:(RCTPromiseResolveBlock)resolve rejecter:(RCTPromiseRejectBlock)reject)
{
  [self resolveWithRustStringArgSymbol:"word_mobile_ios_submit_study_answer" value:requestJson resolver:resolve rejecter:reject];
}

RCT_EXPORT_METHOD(completeStudySession:(NSString *)sessionId resolver:(RCTPromiseResolveBlock)resolve rejecter:(RCTPromiseRejectBlock)reject)
{
  [self resolveWithRustStringArgSymbol:"word_mobile_ios_complete_study_session" value:sessionId resolver:resolve rejecter:reject];
}

RCT_EXPORT_METHOD(cancelStudySession:(NSString *)sessionId resolver:(RCTPromiseResolveBlock)resolve rejecter:(RCTPromiseRejectBlock)reject)
{
  [self resolveWithRustVoidStringArgSymbol:"word_mobile_ios_cancel_study_session" value:sessionId resolver:resolve rejecter:reject];
}

RCT_EXPORT_METHOD(getReportsOverview:(RCTPromiseResolveBlock)resolve rejecter:(RCTPromiseRejectBlock)reject)
{
  (void)resolve;
  [self rejectNotImplemented:@"getReportsOverview" rejecter:reject];
}

RCT_EXPORT_METHOD(getWrongWords:(NSString *)filter resolver:(RCTPromiseResolveBlock)resolve rejecter:(RCTPromiseRejectBlock)reject)
{
  (void)filter;
  (void)resolve;
  [self rejectNotImplemented:@"getWrongWords" rejecter:reject];
}

RCT_EXPORT_METHOD(getWrongWordDetail:(double)entryId resolver:(RCTPromiseResolveBlock)resolve rejecter:(RCTPromiseRejectBlock)reject)
{
  (void)entryId;
  (void)resolve;
  [self rejectNotImplemented:@"getWrongWordDetail" rejecter:reject];
}

RCT_EXPORT_METHOD(getTodayAiPassageContext:(RCTPromiseResolveBlock)resolve rejecter:(RCTPromiseRejectBlock)reject)
{
  (void)resolve;
  [self rejectNotImplemented:@"getTodayAiPassageContext" rejecter:reject];
}

RCT_EXPORT_METHOD(generateAiPassage:(NSString *)requestJson resolver:(RCTPromiseResolveBlock)resolve rejecter:(RCTPromiseRejectBlock)reject)
{
  [self resolveWithRustStringArgSymbol:"word_mobile_ios_generate_ai_passage" value:requestJson resolver:resolve rejecter:reject];
}

RCT_EXPORT_METHOD(getAiPassageHistory:(RCTPromiseResolveBlock)resolve rejecter:(RCTPromiseRejectBlock)reject)
{
  [self resolveWithRustStringSymbol:"word_mobile_ios_get_ai_passage_history" resolver:resolve rejecter:reject];
}

RCT_EXPORT_METHOD(getAiPassage:(NSString *)passageId resolver:(RCTPromiseResolveBlock)resolve rejecter:(RCTPromiseRejectBlock)reject)
{
  [self resolveWithRustStringArgSymbol:"word_mobile_ios_get_ai_passage" value:passageId resolver:resolve rejecter:reject];
}

RCT_EXPORT_METHOD(saveAiPassage:(NSString *)requestJson resolver:(RCTPromiseResolveBlock)resolve rejecter:(RCTPromiseRejectBlock)reject)
{
  [self resolveWithRustVoidStringArgSymbol:"word_mobile_ios_save_ai_passage" value:requestJson resolver:resolve rejecter:reject];
}

@end
