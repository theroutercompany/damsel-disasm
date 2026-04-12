#import <Foundation/Foundation.h>

@protocol GreetingProviding <NSObject>
@property (nonatomic, readonly) NSString *protocolLabel;
+ (NSString *)protocolClassGreeting;
- (NSString *)greeting;
@optional
- (NSString *)optionalGreeting;
@end

@interface Speaker : NSObject
{
    NSString *_prefixSeed;
    NSString *_title;
}
@property (nonatomic, copy) NSString *title;
+ (NSString *)speakerKind;
- (NSString *)prefix;
@end

@implementation Speaker
@synthesize title = _title;

+ (NSString *)speakerKind {
    return @"speaker";
}

- (instancetype)init {
    self = [super init];
    if (self) {
        _prefixSeed = @"hello";
        _title = @"prefix";
    }
    return self;
}

- (NSString *)prefix {
    return [NSString stringWithFormat:@"%@ %@", _prefixSeed, self.title];
}
@end

@interface Greeter : Speaker <GreetingProviding>
{
    NSString *_name;
    NSInteger _emphasis;
}
@property (nonatomic, copy) NSString *name;
@property (nonatomic, assign) NSInteger emphasis;
+ (NSString *)greeterKind;
@end

@implementation Greeter
@synthesize name = _name;
@synthesize emphasis = _emphasis;

+ (NSString *)greeterKind {
    return @"greeter";
}

+ (NSString *)protocolClassGreeting {
    return @"class greeting";
}

- (instancetype)init {
    self = [super init];
    if (self) {
        _name = @"objc";
        _emphasis = 2;
    }
    return self;
}

- (NSString *)protocolLabel {
    return @"greeting-provider";
}

- (NSString *)optionalGreeting {
    return [self greeting];
}

- (NSString *)greeting {
    return [NSString stringWithFormat:@"%@ from %@ (%ld)", [self prefix], self.name, (long)self.emphasis];
}
@end

@protocol Excitement <NSObject>
- (NSString *)excitedGreeting;
+ (NSString *)categoryKind;
@end

@protocol DiagnosticProviding <NSObject>
@required
- (NSString *)diagnosticSummary;
@optional
- (NSString *)diagnosticToken;
@end

@interface Greeter (Excited) <Excitement>
@property (nonatomic, readonly) NSString *categoryToken;
+ (NSString *)categoryKind;
- (NSString *)emphasizedGreeting;
@end

@implementation Greeter (Excited)
- (NSString *)categoryToken {
    return @"category-token";
}

+ (NSString *)categoryKind {
    return @"excited";
}

- (NSString *)emphasizedGreeting {
    return [[self greeting] uppercaseString];
}

- (NSString *)excitedGreeting {
    return [NSString stringWithFormat:@"%@!", [self emphasizedGreeting]];
}
@end

@interface Speaker (Diagnostics) <DiagnosticProviding>
@property (nonatomic, readonly) NSString *diagnosticLabel;
+ (NSString *)diagnosticCategory;
- (NSString *)diagnosticToken;
- (NSString *)diagnosticSummary;
@end

@implementation Speaker (Diagnostics)
- (NSString *)diagnosticLabel {
    return @"speaker-diagnostics";
}

+ (NSString *)diagnosticCategory {
    return @"diagnostics";
}

- (NSString *)diagnosticToken {
    return @"diag";
}

- (NSString *)diagnosticSummary {
    return [NSString stringWithFormat:@"%@ [%@:%@]", [self prefix], self.diagnosticLabel, self.diagnosticToken];
}
@end

@interface Greeter (Metrics)
@property (nonatomic, readonly) NSString *metricsToken;
+ (NSString *)metricsCategory;
- (NSString *)metricsSummary;
- (NSString *)metricsSummaryWithPrefix:(NSString *)prefix;
@end

@implementation Greeter (Metrics)
- (NSString *)metricsToken {
    return @"metrics";
}

+ (NSString *)metricsCategory {
    return @"metrics";
}

- (NSString *)metricsSummary {
    return [NSString stringWithFormat:@"%@:%ld:%@", self.name, (long)self.emphasis, self.metricsToken];
}

- (NSString *)metricsSummaryWithPrefix:(NSString *)prefix {
    return [NSString stringWithFormat:@"%@:%@", prefix, [self metricsSummary]];
}
@end

int main(void) {
    @autoreleasepool {
        Greeter *greeter = [Greeter new];
        NSLog(@"%@", [greeter greeting]);
        NSLog(@"%@", [greeter emphasizedGreeting]);
        NSLog(@"%@", [greeter diagnosticSummary]);
        NSLog(@"%@", [greeter metricsSummary]);
    }
    return 0;
}
