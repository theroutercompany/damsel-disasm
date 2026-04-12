#import <Foundation/Foundation.h>

@protocol GreetingProviding <NSObject>
- (NSString *)greeting;
@end

@interface Speaker : NSObject
- (NSString *)prefix;
@end

@implementation Speaker
- (NSString *)prefix {
    return @"hello";
}
@end

@interface Greeter : Speaker <GreetingProviding>
@property (nonatomic, copy) NSString *name;
@end

@implementation Greeter
- (instancetype)init {
    self = [super init];
    if (self) {
        _name = @"objc";
    }
    return self;
}

- (NSString *)greeting {
    return [NSString stringWithFormat:@"%@ from %@", [self prefix], self.name];
}
@end

@interface Greeter (Excited)
- (NSString *)emphasizedGreeting;
@end

@implementation Greeter (Excited)
- (NSString *)emphasizedGreeting {
    return [[self greeting] uppercaseString];
}
@end

int main(void) {
    @autoreleasepool {
        Greeter *greeter = [Greeter new];
        NSLog(@"%@", [greeter greeting]);
        NSLog(@"%@", [greeter emphasizedGreeting]);
    }
    return 0;
}
