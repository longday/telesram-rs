#import <AppKit/AppKit.h>
#import <WebKit/WebKit.h>
#import <objc/runtime.h>

// Exercise WebKit's host and resource-type boundaries without contacting trackers.
static NSURL *ruleStoreDirectory;

static BOOL cleanupStore(void) {
    if (!ruleStoreDirectory) return YES;
    NSError *error = nil;
    BOOL removed = [[NSFileManager defaultManager] removeItemAtURL:ruleStoreDirectory error:&error];
    if (!removed) fprintf(stderr, "NATIVE FILTER cleanup failed: %s\n", error.description.UTF8String);
    ruleStoreDirectory = nil;
    return removed;
}

static IMP originalHandlesScheme;
static BOOL probeHandlesScheme(id self, SEL selector, NSString *scheme) {
    if ([scheme caseInsensitiveCompare:@"https"] == NSOrderedSame ||
        [scheme caseInsensitiveCompare:@"http"] == NSOrderedSame) return NO;
    return ((BOOL (*)(id, SEL, NSString *))originalHandlesScheme)(self, selector, scheme);
}

static void die(NSString *reason) {
    fprintf(stderr, "NATIVE FILTER FAIL: %s\n", reason.UTF8String);
    fflush(stderr);
    cleanupStore();
    exit(2);
}

@interface FilterProbe : NSObject <WKURLSchemeHandler, WKNavigationDelegate>
@property (nonatomic, strong) WKWebView *webView;
@property (nonatomic, strong) NSWindow *window;
@property (nonatomic, strong) WKContentRuleList *rules;
@property (nonatomic, strong) NSMutableSet<NSString *> *observed;
@property (nonatomic, copy) void (^navigationDone)(void);
@end

@implementation FilterProbe

- (void)webView:(WKWebView *)webView startURLSchemeTask:(id<WKURLSchemeTask>)task {
    NSURL *url = task.request.URL;
    if (![@[@"https", @"http"] containsObject:url.scheme.lowercaseString])
        die([NSString stringWithFormat:@"Unexpected scheme: %@", url]);
    NSURLComponents *parts = [NSURLComponents componentsWithURL:url resolvingAgainstBaseURL:NO];
    NSString *token;
    for (NSURLQueryItem *item in parts.queryItems) {
        if ([item.name isEqualToString:@"probe"]) token = item.value;
    }
    if (!token) die([NSString stringWithFormat:@"Unlabeled request (fixture escaped): %@", url]);
    @synchronized (self.observed) { [self.observed addObject:token]; }
    BOOL document = [url.path isEqualToString:@"/probe-document"];
    NSString *body = document ? @"<!doctype html><title>fixture-document</title><main id='document-marker'>navigable</main>" :
        @"<svg xmlns='http://www.w3.org/2000/svg' width='1' height='1'><rect width='1' height='1'/></svg>";
    NSData *data = [body dataUsingEncoding:NSUTF8StringEncoding];
    NSHTTPURLResponse *response = [[NSHTTPURLResponse alloc] initWithURL:url statusCode:200 HTTPVersion:@"HTTP/1.1"
        headerFields:@{ @"Content-Type": document ? @"text/html; charset=utf-8" : @"image/svg+xml",
                        @"Access-Control-Allow-Origin": @"*", @"Cache-Control": @"no-store" }];
    [task didReceiveResponse:response];
    [task didReceiveData:data];
    [task didFinish];
}

- (void)webView:(WKWebView *)webView stopURLSchemeTask:(id<WKURLSchemeTask>)task {}

- (void)webView:(WKWebView *)webView didFinishNavigation:(WKNavigation *)navigation {
    if (!self.navigationDone) die(@"Unexpected navigation completion");
    void (^done)(void) = self.navigationDone;
    self.navigationDone = nil;
    done();
}

- (void)webView:(WKWebView *)webView didFailProvisionalNavigation:(WKNavigation *)navigation withError:(NSError *)error {
    die([NSString stringWithFormat:@"Provisional navigation failed: %@", error]);
}

- (void)webView:(WKWebView *)webView didFailNavigation:(WKNavigation *)navigation withError:(NSError *)error {
    die([NSString stringWithFormat:@"Navigation failed: %@", error]);
}

- (void)navigate:(NSURL *)url simulated:(BOOL)simulated then:(void (^)(void))done {
    if (self.navigationDone) die(@"Overlapping navigations");
    self.navigationDone = done;
    if (simulated) {
        NSString *html = @"<!doctype html><meta charset='utf-8'><div class='yamb-global-bar' id='bar'>bar</div><div id='neighbor'>neighbor</div>";
        if (![self.webView loadSimulatedRequest:[NSURLRequest requestWithURL:url] responseHTMLString:html])
            die([NSString stringWithFormat:@"Could not simulate document: %@", url]);
    } else if (![self.webView loadRequest:[NSURLRequest requestWithURL:url]]) {
        die([NSString stringWithFormat:@"Could not load document: %@", url]);
    }
}

- (void)evaluate:(NSString *)script arguments:(NSDictionary *)arguments then:(void (^)(id))done {
    [self.webView callAsyncJavaScript:script arguments:arguments inFrame:nil inContentWorld:WKContentWorld.pageWorld
        completionHandler:^(id value, NSError *error) {
            if (error) die([NSString stringWithFormat:@"JavaScript failed: %@", error]);
            done(value);
        }];
}

- (BOOL)saw:(NSString *)token {
    @synchronized (self.observed) { return [self.observed containsObject:token]; }
}

// Expectations are handwritten and never read or inferred from managed-rules.json.
- (NSArray<NSDictionary *> *)casesForPhase:(NSUInteger)phase {
    NSArray<NSArray *> *entries = @[
        @[@"mc", @"https://mc.yandex.ru/watch", @YES],
        @[@"mc-subdomain", @"https://a.mc.yandex.ru/watch", @YES],
        @[@"mc-uppercase-port", @"https://MC.YANDEX.RU:8443/WATCH", @YES],
        @[@"webvisor", @"https://mc.webvisor.org/watch", @YES],
        @[@"adfox", @"https://adfox.ru/banner", @YES],
        @[@"analytics", @"https://analytics.mobile.yandex.net/collect", @YES],
        @[@"clck", @"https://yandex.ru/clck/redirect", @YES],
        @[@"clck-port", @"https://yandex.ru:8443/CLCK/redirect", @YES],
        @[@"log-query", @"https://yandex.ru/log?event=1", @YES],
        @[@"ya-clck", @"https://ya.ru/clck/click", @YES],
        @[@"metrica-local", @"https://yandexmetrica.com/watch", @YES],
        @[@"google", @"https://www.google-analytics.com/collect", @NO],
        @[@"example", @"https://example.test/pixel", @NO],
        @[@"lookalike", @"https://mc.yandex.ru.evil.test/watch", @NO],
        @[@"query-only", @"https://example.test/pixel?next=mc.yandex.ru/watch", @NO],
        @[@"telemost-threads", @"https://telemost.yandex.ru/threads/fixture", @NO],
        @[@"telemost-api", @"https://telemost.yandex.ru/api/fixture", @NO],
        @[@"telemost-360-api", @"https://telemost.360.yandex.ru/api/fixture", @NO],
        @[@"passport", @"https://passport.yandex.ru/auth/fixture", @NO],
        @[@"id", @"https://id.yandex.ru/auth/fixture", @NO],
        @[@"cookier", @"https://cookier.360.yandex.ru/auth/fixture", @NO],
        @[@"yastatic", @"https://yastatic.net/static/fixture.svg", @NO],
        @[@"chat-static", @"https://chat-static.s3.yandex.net/fixture.svg", @NO],
        @[@"clck-subdomain", @"https://sub.yandex.ru/clck/redirect", @NO],
        @[@"clck-lookalike", @"https://yandex.ru.evil.test/clck/redirect", @NO],
        @[@"ordinary-yandex", @"https://yandex.ru/threads/fixture", @NO]
    ];
    NSMutableArray *cases = [NSMutableArray arrayWithCapacity:entries.count];
    for (NSUInteger index = 0; index < entries.count; index++) {
        NSArray *entry = entries[index];
        NSString *token = [NSString stringWithFormat:@"p%lu-%lu", (unsigned long)phase, (unsigned long)index];
        NSString *separator = [entry[1] containsString:@"?"] ? @"&" : @"?";
        [cases addObject:@{ @"name": entry[0], @"url": [NSString stringWithFormat:@"%@%@probe=%@", entry[1], separator, token],
                            @"blocked": entry[2], @"token": token }];
    }
    return cases;
}

- (void)checkResourcesForPhase:(NSUInteger)phase raw:(BOOL)raw then:(void (^)(void))done {
    NSArray<NSDictionary *> *cases = [self casesForPhase:phase + (raw ? 10 : 0)];
    NSMutableArray *urls = [NSMutableArray arrayWithCapacity:cases.count];
    for (NSDictionary *entry in cases) [urls addObject:entry[@"url"]];
    NSString *script = raw
        ? @"return await Promise.all(urls.map(url => fetch(url, {cache:'no-store'}).then(response => response.status === 200 ? 'load' : 'status-error').catch(() => 'error')));"
        : @"window.__probeImages = []; "
        @"return await Promise.all(urls.map(url => new Promise(resolve => { "
        @"const image = new Image(); window.__probeImages.push(image); let settled = false; "
        @"const timeout = setTimeout(() => { if (!settled) { settled = true; resolve('timeout'); } }, 5000); "
        @"const finish = result => { if (!settled) { settled = true; clearTimeout(timeout); resolve(result); } }; "
        @"image.onload = () => finish('load'); image.onerror = () => finish('error'); image.src = url; "
        @"})));";
    [self evaluate:script arguments:@{ @"urls": urls } then:^(id value) {
        if (![value isKindOfClass:NSArray.class] || [value count] != cases.count)
            die([NSString stringWithFormat:@"Invalid image results: %@", value]);
        for (NSUInteger index = 0; index < cases.count; index++) {
            NSDictionary *entry = cases[index];
            BOOL blocked = phase == 1 && [entry[@"blocked"] boolValue];
            NSString *expected = blocked ? @"error" : @"load";
            BOOL observed = [self saw:entry[@"token"]];
            if (![value[index] isEqual:expected] || observed == blocked)
                die([NSString stringWithFormat:@"phase %lu %@: JS=%@ handler=%d; expected JS=%@ handler=%d",
                     (unsigned long)phase, entry[@"name"], value[index], observed, expected, !blocked]);
        }
        fprintf(stderr, "NATIVE FILTER phase %lu: %lu %s requests checked in JS and handler\n",
                (unsigned long)phase, (unsigned long)cases.count, raw ? "fetch" : "image");
        done();
    }];
}

- (void)checkCSSForPhase:(NSUInteger)phase index:(NSUInteger)index then:(void (^)(void))done {
    NSArray<NSString *> *origins = @[@"https://telemost.yandex.ru", @"https://telemost.360.yandex.ru",
        @"https://example.test", @"https://telemost.yandex.ru.evil.test", @"http://telemost.yandex.ru"];
    if (index == origins.count) { done(); return; }
    BOOL hidden = phase == 1 && index < 2;
    NSURL *url = [NSURL URLWithString:[NSString stringWithFormat:@"%@/threads/fixture?probe=css-%lu-%lu",
                                      origins[index], (unsigned long)phase, (unsigned long)index]];
    [self navigate:url simulated:YES then:^{
        [self evaluate:@"return [getComputedStyle(document.getElementById('bar')).display, getComputedStyle(document.getElementById('neighbor')).display];"
              arguments:@{} then:^(id value) {
            if (![value isKindOfClass:NSArray.class] || [value count] != 2 ||
                ![value[0] isEqual:hidden ? @"none" : @"block"] || ![value[1] isEqual:@"block"])
                die([NSString stringWithFormat:@"CSS phase %lu origin %@: %@ (expected bar %@, neighbor block)",
                     (unsigned long)phase, origins[index], value, hidden ? @"none" : @"block"]);
            [self checkCSSForPhase:phase index:index + 1 then:done];
        }];
    }];
}

- (void)checkDocumentsForPhase:(NSUInteger)phase index:(NSUInteger)index then:(void (^)(void))done {
    NSArray<NSString *> *hosts = @[@"mc.yandex.ru", @"mc.webvisor.org", @"adfox.ru"];
    if (index == hosts.count) { done(); return; }
    NSString *token = [NSString stringWithFormat:@"doc-%lu-%lu", (unsigned long)phase, (unsigned long)index];
    NSURL *url = [NSURL URLWithString:[NSString stringWithFormat:@"https://%@/probe-document?probe=%@", hosts[index], token]];
    [self navigate:url simulated:NO then:^{
        [self evaluate:@"return document.getElementById('document-marker')?.textContent ?? null;" arguments:@{} then:^(id value) {
            if (![value isEqual:@"navigable"] || ![self saw:token])
                die([NSString stringWithFormat:@"Tracker document did not navigate through handler: %@ / %@", url, value]);
            [self checkDocumentsForPhase:phase index:index + 1 then:done];
        }];
    }];
}

- (void)runPhase:(NSUInteger)phase {
    if (phase == 3) {
        if (!cleanupStore()) exit(2);
        fprintf(stderr, "NATIVE FILTER PASS: off -> on -> off; exact compiled list, local WebKit loads only\n");
        fflush(stderr);
        exit(0);
    }
    WKUserContentController *controller = self.webView.configuration.userContentController;
    [controller removeAllContentRuleLists];
    if (phase == 1) [controller addContentRuleList:self.rules];
    [self checkCSSForPhase:phase index:0 then:^{
        NSURL *url = [NSURL URLWithString:[NSString stringWithFormat:@"https://telemost.yandex.ru/threads/images?probe=page-%lu",
                                            (unsigned long)phase]];
        [self navigate:url simulated:YES then:^{
            [self checkResourcesForPhase:phase raw:NO then:^{
                [self checkResourcesForPhase:phase raw:YES then:^{
                    [self checkDocumentsForPhase:phase index:0 then:^{ [self runPhase:phase + 1]; }];
                }];
            }];
        }];
    }];
}

- (void)start {
    Method method = class_getClassMethod(WKWebView.class, @selector(handlesURLScheme:));
    if (!method) die(@"WKWebView +handlesURLScheme: unavailable; cannot install local HTTPS handler");
    originalHandlesScheme = method_setImplementation(method, (IMP)probeHandlesScheme);
    if (!originalHandlesScheme) die(@"Cannot swizzle WebKit scheme check");

    NSString *directory = [NSTemporaryDirectory() stringByAppendingPathComponent:[[NSUUID UUID] UUIDString]];
    NSURL *directoryURL = [NSURL fileURLWithPath:directory isDirectory:YES];
    NSError *error = nil;
    if (![[NSFileManager defaultManager] createDirectoryAtURL:directoryURL withIntermediateDirectories:NO attributes:nil error:&error])
        die([NSString stringWithFormat:@"Cannot create isolated rule store: %@", error]);
    ruleStoreDirectory = directoryURL;
    NSString *json = [NSString stringWithContentsOfFile:@"assets/managed-rules.json" encoding:NSUTF8StringEncoding error:&error];
    if (!json) die([NSString stringWithFormat:@"Cannot read exact managed rules: %@", error]);

    self.observed = [NSMutableSet set];
    WKWebViewConfiguration *configuration = [WKWebViewConfiguration new];
    configuration.websiteDataStore = [WKWebsiteDataStore nonPersistentDataStore];
    [configuration setURLSchemeHandler:self forURLScheme:@"https"];
    [configuration setURLSchemeHandler:self forURLScheme:@"http"];
    if ([configuration urlSchemeHandlerForURLScheme:@"https"] != self ||
        [configuration urlSchemeHandlerForURLScheme:@"http"] != self)
        die(@"WebKit rejected local HTTPS/HTTP scheme handler");
    self.webView = [[WKWebView alloc] initWithFrame:NSMakeRect(0, 0, 640, 480) configuration:configuration];
    self.webView.navigationDelegate = self;
    self.window = [[NSWindow alloc] initWithContentRect:NSMakeRect(0, 0, 640, 480)
        styleMask:NSWindowStyleMaskBorderless backing:NSBackingStoreBuffered defer:NO];
    [self.window.contentView addSubview:self.webView];

    WKContentRuleListStore *store = [WKContentRuleListStore storeWithURL:ruleStoreDirectory];
    [store compileContentRuleListForIdentifier:@"yandex-filter-native-probe" encodedContentRuleList:json
        completionHandler:^(WKContentRuleList *list, NSError *compileError) {
            if (!list || compileError) die([NSString stringWithFormat:@"Native rule compilation failed: %@", compileError]);
            self.rules = list;
            [self runPhase:0];
        }];
}
@end

static FilterProbe *probe;
int main(void) {
    @autoreleasepool {
        [NSApplication sharedApplication];
        [NSApp setActivationPolicy:NSApplicationActivationPolicyProhibited];
        probe = [FilterProbe new];
        dispatch_after(dispatch_time(DISPATCH_TIME_NOW, (int64_t)(29 * NSEC_PER_SEC)), dispatch_get_main_queue(), ^{
            die(@"Timed out after 29 seconds; WebKit may not dispatch custom HTTPS/HTTP handlers on this OS");
        });
        [probe start];
        [[NSRunLoop mainRunLoop] run];
    }
    return 2;
}
