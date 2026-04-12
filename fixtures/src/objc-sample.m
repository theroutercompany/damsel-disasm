#import <Foundation/Foundation.h>

@interface Greeter : NSObject
- (NSString *)greeting;
@end

@implementation Greeter
- (NSString *)greeting {
    return @"hello from objc";
}
@end

int main(void) {
    @autoreleasepool {
        Greeter *greeter = [Greeter new];
        NSLog(@"%@", [greeter greeting]);
    }
    return 0;
}
