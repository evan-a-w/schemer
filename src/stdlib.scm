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

(defmacro (while condi expr) 
     (let ((WHILE_GO (open-lambda ()
          (if condi
              (begin
                  expr
                  (WHILE_GO))
              '()))))
          (WHILE_GO)))

(defmacro (for i in l expr)
          (let ((i '())
                (FOR_GO (lambda (FOR__ACC, FOR__CURR)
                        (begin (set! i FOR__CURR) expr))))
                (foldl FOR_GO '() l)))
