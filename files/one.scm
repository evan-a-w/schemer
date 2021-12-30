(foldl cons '() '(1 2 3 4 5))

(define (foldl func accu alist)
    (if (null? alist)
    accu
    (foldl func (func (car alist) accu) (cdr alist))))

(foldl cons '() '(1 2 3 4 5))
