(define (even? x) 
        (= (modulo x 2) 0))

(define (vector->list vec)
        (foldr cons '() vec))

(define (list->vector list)
        (foldl vector-append! #() list))

(define (do func init lim)
        (let ((go (lambda (var)
                          (if (= var lim)
                              (func var)
                              (begin
                                (func var)
                                (go (+ var 1)))))))
        (go init)))

(define (while pred symbol func)
        (let ((go (lambda () (if (pred (sym->id symbol))
                                 (begin
                                   (func)
                                   (go))
                                 '()))))
        (go)))
        
